//! Native Windows WASAPI audio HAL implementation using CPAL.

use crate::error::{Result, VoiceRuntimeError};
use crate::hal::traits::{AudioDeviceManager, AudioHal, AudioInputStream, AudioOutputStream};
use crate::hal::types::{AudioDeviceDirection, AudioDeviceInfo, AudioStreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Native Windows Audio Hardware Abstraction Layer backed by WASAPI via CPAL.
pub struct CpalAudioHal {
    host: Host,
}

impl std::fmt::Debug for CpalAudioHal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalAudioHal")
            .field("host_id", &self.host.id())
            .finish()
    }
}

impl Default for CpalAudioHal {
    fn default() -> Self {
        Self::new()
    }
}

impl CpalAudioHal {
    /// Initializes a new [`CpalAudioHal`] on the default host (Windows WASAPI).
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    fn find_device(
        &self,
        name_filter: Option<&str>,
        direction: AudioDeviceDirection,
    ) -> Result<Device> {
        let devices =
            match direction {
                AudioDeviceDirection::Input => self.host.input_devices().map_err(|e| {
                    VoiceRuntimeError::AudioDeviceInitFailed {
                        message: format!("Failed to enumerate input devices: {e}"),
                    }
                })?,
                AudioDeviceDirection::Output => self.host.output_devices().map_err(|e| {
                    VoiceRuntimeError::AudioDeviceInitFailed {
                        message: format!("Failed to enumerate output devices: {e}"),
                    }
                })?,
            };

        if let Some(target_name) = name_filter {
            for dev in devices {
                let name = dev
                    .description()
                    .map(|d| d.name().to_string())
                    .unwrap_or_else(|_| dev.to_string());
                if name.to_lowercase().contains(&target_name.to_lowercase()) {
                    return Ok(dev);
                }
            }
            return Err(VoiceRuntimeError::AudioDeviceNotFound {
                name: target_name.to_string(),
            });
        }

        // Return default device if no filter provided
        match direction {
            AudioDeviceDirection::Input => self.host.default_input_device().ok_or_else(|| {
                VoiceRuntimeError::AudioDeviceNotFound {
                    name: "Default Windows input device".to_string(),
                }
            }),
            AudioDeviceDirection::Output => self.host.default_output_device().ok_or_else(|| {
                VoiceRuntimeError::AudioDeviceNotFound {
                    name: "Default Windows output device".to_string(),
                }
            }),
        }
    }

    fn extract_device_info(
        &self,
        dev: &Device,
        direction: AudioDeviceDirection,
        is_default: bool,
    ) -> Option<AudioDeviceInfo> {
        let name = dev
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| dev.to_string());
        let id = dev
            .id()
            .map(|id| format!("{id:?}"))
            .unwrap_or_else(|_| name.clone());

        let (default_sr, default_ch) = match direction {
            AudioDeviceDirection::Input => dev
                .default_input_config()
                .map(|c| (c.sample_rate(), c.channels()))
                .unwrap_or((48000, 2)),
            AudioDeviceDirection::Output => dev
                .default_output_config()
                .map(|c| (c.sample_rate(), c.channels()))
                .unwrap_or((48000, 2)),
        };

        let mut supported_rates = Vec::new();
        match direction {
            AudioDeviceDirection::Input => {
                if let Ok(supported) = dev.supported_input_configs() {
                    for range in supported {
                        let min = range.min_sample_rate();
                        let max = range.max_sample_rate();
                        if !supported_rates.contains(&min) {
                            supported_rates.push(min);
                        }
                        if !supported_rates.contains(&max) {
                            supported_rates.push(max);
                        }
                    }
                }
            }
            AudioDeviceDirection::Output => {
                if let Ok(supported) = dev.supported_output_configs() {
                    for range in supported {
                        let min = range.min_sample_rate();
                        let max = range.max_sample_rate();
                        if !supported_rates.contains(&min) {
                            supported_rates.push(min);
                        }
                        if !supported_rates.contains(&max) {
                            supported_rates.push(max);
                        }
                    }
                }
            }
        }
        supported_rates.sort_unstable();

        Some(AudioDeviceInfo {
            id,
            name,
            is_default,
            direction,
            default_sample_rate: default_sr,
            default_channels: default_ch,
            supported_sample_rates: supported_rates,
        })
    }
}

impl AudioDeviceManager for CpalAudioHal {
    fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let default_id = self.host.default_input_device().and_then(|d| d.id().ok());
        let devices =
            self.host
                .input_devices()
                .map_err(|e| VoiceRuntimeError::AudioDeviceInitFailed {
                    message: format!("Input devices enumeration failed: {e}"),
                })?;

        let mut list = Vec::new();
        for dev in devices {
            let is_default = default_id
                .as_ref()
                .map_or(false, |did| dev.id().ok().as_ref() == Some(did));
            if let Some(info) =
                self.extract_device_info(&dev, AudioDeviceDirection::Input, is_default)
            {
                list.push(info);
            }
        }
        Ok(list)
    }

    fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let default_id = self.host.default_output_device().and_then(|d| d.id().ok());
        let devices =
            self.host
                .output_devices()
                .map_err(|e| VoiceRuntimeError::AudioDeviceInitFailed {
                    message: format!("Output devices enumeration failed: {e}"),
                })?;

        let mut list = Vec::new();
        for dev in devices {
            let is_default = default_id
                .as_ref()
                .map_or(false, |did| dev.id().ok().as_ref() == Some(did));
            if let Some(info) =
                self.extract_device_info(&dev, AudioDeviceDirection::Output, is_default)
            {
                list.push(info);
            }
        }
        Ok(list)
    }

    fn default_input_device(&self) -> Result<AudioDeviceInfo> {
        let dev = self.host.default_input_device().ok_or_else(|| {
            VoiceRuntimeError::AudioDeviceNotFound {
                name: "Default Windows input device".to_string(),
            }
        })?;
        self.extract_device_info(&dev, AudioDeviceDirection::Input, true)
            .ok_or_else(|| VoiceRuntimeError::AudioDeviceInitFailed {
                message: "Failed to extract default input device info".to_string(),
            })
    }

    fn default_output_device(&self) -> Result<AudioDeviceInfo> {
        let dev = self.host.default_output_device().ok_or_else(|| {
            VoiceRuntimeError::AudioDeviceNotFound {
                name: "Default Windows output device".to_string(),
            }
        })?;
        self.extract_device_info(&dev, AudioDeviceDirection::Output, true)
            .ok_or_else(|| VoiceRuntimeError::AudioDeviceInitFailed {
                message: "Failed to extract default output device info".to_string(),
            })
    }
}

/// Active CPAL WASAPI Microphone Input Capture Stream.
pub struct CpalAudioInputStream {
    _stream: Mutex<Option<cpal::Stream>>,
    receiver: Mutex<Receiver<Vec<f32>>>,
    config: AudioStreamConfig,
    is_running: Arc<AtomicBool>,
}

impl AudioInputStream for CpalAudioInputStream {
    fn start(&mut self) -> Result<()> {
        if let Ok(guard) = self._stream.lock() {
            if let Some(stream) = guard.as_ref() {
                stream
                    .play()
                    .map_err(|e| VoiceRuntimeError::AudioStreamStartupFailed {
                        message: format!("Failed to start input audio stream: {e}"),
                    })?;
                self.is_running.store(true, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Ok(guard) = self._stream.lock() {
            if let Some(stream) = guard.as_ref() {
                let _ = stream.pause();
                self.is_running.store(false, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    fn capture_samples(&mut self) -> Result<Vec<f32>> {
        let mut all_samples = Vec::new();
        if let Ok(rx) = self.receiver.lock() {
            while let Ok(chunk) = rx.try_recv() {
                all_samples.extend(chunk);
            }
        }
        Ok(all_samples)
    }

    fn config(&self) -> AudioStreamConfig {
        self.config
    }
}

/// Active CPAL WASAPI Speaker/Headphone Playback Stream.
pub struct CpalAudioOutputStream {
    _stream: Mutex<Option<cpal::Stream>>,
    buffer: Arc<Mutex<VecDeque<f32>>>,
    config: AudioStreamConfig,
    is_running: Arc<AtomicBool>,
    samples_written: Arc<AtomicUsize>,
    samples_played: Arc<AtomicUsize>,
}

impl AudioOutputStream for CpalAudioOutputStream {
    fn start(&mut self) -> Result<()> {
        if let Ok(guard) = self._stream.lock() {
            if let Some(stream) = guard.as_ref() {
                stream
                    .play()
                    .map_err(|e| VoiceRuntimeError::AudioStreamStartupFailed {
                        message: format!("Failed to start output audio stream: {e}"),
                    })?;
                self.is_running.store(true, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    fn write_chunk(&mut self, samples: &[f32]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }
        if let Ok(mut buf) = self.buffer.lock() {
            buf.extend(samples.iter().copied());
            self.samples_written
                .fetch_add(samples.len(), Ordering::SeqCst);
        }
        Ok(())
    }

    fn flush_and_wait(&mut self) -> Result<()> {
        let total_written = self.samples_written.load(Ordering::SeqCst);
        let timeout = Duration::from_millis(5000);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            let played = self.samples_played.load(Ordering::SeqCst);
            let is_empty = self.buffer.lock().map_or(true, |b| b.is_empty());
            if is_empty && played >= total_written {
                // Short grace sleep for hardware DAC to finish output buffer
                std::thread::sleep(Duration::from_millis(50));
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Ok(guard) = self._stream.lock() {
            if let Some(stream) = guard.as_ref() {
                let _ = stream.pause();
                self.is_running.store(false, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    fn config(&self) -> AudioStreamConfig {
        self.config
    }
}

impl AudioHal for CpalAudioHal {
    fn open_input_stream(
        &self,
        device_name: Option<&str>,
        requested_config: &AudioStreamConfig,
    ) -> Result<Box<dyn AudioInputStream>> {
        let device = self.find_device(device_name, AudioDeviceDirection::Input)?;
        let default_config = device.default_input_config().map_err(|e| {
            VoiceRuntimeError::AudioDeviceInitFailed {
                message: format!("Failed to query default input config: {e}"),
            }
        })?;

        let sample_format = default_config.sample_format();
        let hw_sample_rate = if requested_config.sample_rate != 0 {
            requested_config.sample_rate
        } else {
            default_config.sample_rate()
        };
        let hw_channels = default_config.channels();

        let stream_config = cpal::StreamConfig {
            channels: hw_channels,
            sample_rate: hw_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        // Bounded channel to transfer raw audio from callback to consumer thread
        let (tx, rx) = sync_channel::<Vec<f32>>(128);
        let is_running = Arc::new(AtomicBool::new(false));
        let running_flag = Arc::clone(&is_running);

        let err_fn = |err| {
            eprintln!("[WASAPI Audio Input Stream Error]: {err}");
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let tx = tx.clone();
                let running = Arc::clone(&running_flag);
                device.build_input_stream(
                    stream_config.clone(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if running.load(Ordering::Relaxed) {
                            // Lightweight non-blocking send; drop frame if consumer is stalled
                            let _ = tx.try_send(data.to_vec());
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let tx = tx.clone();
                let running = Arc::clone(&running_flag);
                device.build_input_stream(
                    stream_config.clone(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if running.load(Ordering::Relaxed) {
                            let f32_chunk: Vec<f32> =
                                data.iter().map(|&s| s as f32 / 32768.0).collect();
                            let _ = tx.try_send(f32_chunk);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let tx = tx.clone();
                let running = Arc::clone(&running_flag);
                device.build_input_stream(
                    stream_config.clone(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if running.load(Ordering::Relaxed) {
                            let f32_chunk: Vec<f32> = data
                                .iter()
                                .map(|&s| (s as f32 - 32768.0) / 32768.0)
                                .collect();
                            let _ = tx.try_send(f32_chunk);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(VoiceRuntimeError::UnsupportedAudioFormat {
                    message: format!("Unsupported CPAL input format: {other:?}"),
                });
            }
        }
        .map_err(|e| VoiceRuntimeError::AudioStreamStartupFailed {
            message: format!("Failed to build input stream: {e}"),
        })?;

        let negotiated_config = AudioStreamConfig {
            sample_rate: hw_sample_rate,
            channels: hw_channels,
            buffer_size: requested_config.buffer_size,
        };

        Ok(Box::new(CpalAudioInputStream {
            _stream: Mutex::new(Some(stream)),
            receiver: Mutex::new(rx),
            config: negotiated_config,
            is_running,
        }))
    }

    fn open_output_stream(
        &self,
        device_name: Option<&str>,
        requested_config: &AudioStreamConfig,
    ) -> Result<Box<dyn AudioOutputStream>> {
        let device = self.find_device(device_name, AudioDeviceDirection::Output)?;
        let default_config = device.default_output_config().map_err(|e| {
            VoiceRuntimeError::AudioDeviceInitFailed {
                message: format!("Failed to query default output config: {e}"),
            }
        })?;

        let sample_format = default_config.sample_format();
        let hw_sample_rate = if requested_config.sample_rate != 0 {
            requested_config.sample_rate
        } else {
            default_config.sample_rate()
        };
        let hw_channels = default_config.channels();

        let stream_config = cpal::StreamConfig {
            channels: hw_channels,
            sample_rate: hw_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = Arc::new(Mutex::new(VecDeque::<f32>::with_capacity(32768)));
        let is_running = Arc::new(AtomicBool::new(false));
        let samples_written = Arc::new(AtomicUsize::new(0));
        let samples_played = Arc::new(AtomicUsize::new(0));

        let err_fn = |err| {
            eprintln!("[WASAPI Audio Output Stream Error]: {err}");
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let buf_clone = Arc::clone(&buffer);
                let running_clone = Arc::clone(&is_running);
                let played_clone = Arc::clone(&samples_played);

                device.build_output_stream(
                    stream_config.clone(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if !running_clone.load(Ordering::Relaxed) {
                            data.fill(0.0);
                            return;
                        }

                        let mut count = 0usize;
                        if let Ok(mut buf) = buf_clone.try_lock() {
                            for sample in data.iter_mut() {
                                if let Some(s) = buf.pop_front() {
                                    *sample = s;
                                    count += 1;
                                } else {
                                    *sample = 0.0;
                                }
                            }
                        } else {
                            data.fill(0.0);
                        }

                        if count > 0 {
                            played_clone.fetch_add(count, Ordering::Relaxed);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let buf_clone = Arc::clone(&buffer);
                let running_clone = Arc::clone(&is_running);
                let played_clone = Arc::clone(&samples_played);

                device.build_output_stream(
                    stream_config.clone(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        if !running_clone.load(Ordering::Relaxed) {
                            data.fill(0);
                            return;
                        }

                        let mut count = 0usize;
                        if let Ok(mut buf) = buf_clone.try_lock() {
                            for sample in data.iter_mut() {
                                if let Some(s) = buf.pop_front() {
                                    *sample = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                                    count += 1;
                                } else {
                                    *sample = 0;
                                }
                            }
                        } else {
                            data.fill(0);
                        }

                        if count > 0 {
                            played_clone.fetch_add(count, Ordering::Relaxed);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(VoiceRuntimeError::UnsupportedAudioFormat {
                    message: format!("Unsupported CPAL output format: {other:?}"),
                });
            }
        }
        .map_err(|e| VoiceRuntimeError::AudioStreamStartupFailed {
            message: format!("Failed to build output stream: {e}"),
        })?;

        let negotiated_config = AudioStreamConfig {
            sample_rate: hw_sample_rate,
            channels: hw_channels,
            buffer_size: requested_config.buffer_size,
        };

        Ok(Box::new(CpalAudioOutputStream {
            _stream: Mutex::new(Some(stream)),
            buffer,
            config: negotiated_config,
            is_running,
            samples_written,
            samples_played,
        }))
    }
}
