//! Deterministic Mock Audio HAL for unit testing and CI without physical hardware.

use crate::error::Result;
use crate::hal::traits::{AudioDeviceManager, AudioHal, AudioInputStream, AudioOutputStream};
use crate::hal::types::{AudioDeviceDirection, AudioDeviceInfo, AudioStreamConfig};
use std::sync::{Arc, Mutex};

/// In-memory deterministic mock audio HAL.
#[derive(Clone, Debug)]
pub struct MockAudioHal {
    pub input_devices: Vec<AudioDeviceInfo>,
    pub output_devices: Vec<AudioDeviceInfo>,
    pub captured_samples: Arc<Mutex<Vec<f32>>>,
    pub played_samples: Arc<Mutex<Vec<f32>>>,
}

impl Default for MockAudioHal {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAudioHal {
    /// Creates a default mock audio HAL with preconfigured mock devices.
    pub fn new() -> Self {
        let input_dev = AudioDeviceInfo {
            id: "mock-input-mic".to_string(),
            name: "Mock Digital Microphone".to_string(),
            is_default: true,
            direction: AudioDeviceDirection::Input,
            default_sample_rate: 16000,
            default_channels: 1,
            supported_sample_rates: vec![16000, 44100, 48000],
        };

        let output_dev = AudioDeviceInfo {
            id: "mock-output-speaker".to_string(),
            name: "Mock Stereo Speaker".to_string(),
            is_default: true,
            direction: AudioDeviceDirection::Output,
            default_sample_rate: 48000,
            default_channels: 2,
            supported_sample_rates: vec![16000, 44100, 48000],
        };

        Self {
            input_devices: vec![input_dev],
            output_devices: vec![output_dev],
            captured_samples: Arc::new(Mutex::new(Vec::new())),
            played_samples: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Sets deterministic samples to be returned by mock input stream capture.
    pub fn set_input_samples(&self, samples: Vec<f32>) {
        if let Ok(mut lock) = self.captured_samples.lock() {
            *lock = samples;
        }
    }

    /// Returns all audio samples written to mock output stream.
    pub fn get_played_samples(&self) -> Vec<f32> {
        self.played_samples.lock().map_or(Vec::new(), |b| b.clone())
    }
}

impl AudioDeviceManager for MockAudioHal {
    fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        Ok(self.input_devices.clone())
    }

    fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        Ok(self.output_devices.clone())
    }

    fn default_input_device(&self) -> Result<AudioDeviceInfo> {
        self.input_devices.first().cloned().ok_or_else(|| {
            crate::error::VoiceRuntimeError::AudioDeviceNotFound {
                name: "Mock default input device".to_string(),
            }
        })
    }

    fn default_output_device(&self) -> Result<AudioDeviceInfo> {
        self.output_devices.first().cloned().ok_or_else(|| {
            crate::error::VoiceRuntimeError::AudioDeviceNotFound {
                name: "Mock default output device".to_string(),
            }
        })
    }
}

/// Active Mock Input Stream.
pub struct MockAudioInputStream {
    samples: Arc<Mutex<Vec<f32>>>,
    config: AudioStreamConfig,
    is_active: bool,
}

impl AudioInputStream for MockAudioInputStream {
    fn start(&mut self) -> Result<()> {
        self.is_active = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.is_active = false;
        Ok(())
    }

    fn capture_samples(&mut self) -> Result<Vec<f32>> {
        if !self.is_active {
            return Ok(Vec::new());
        }
        let mut lock = self.samples.lock().unwrap();
        let samples = lock.clone();
        lock.clear();
        Ok(samples)
    }

    fn config(&self) -> AudioStreamConfig {
        self.config
    }
}

/// Active Mock Output Stream.
pub struct MockAudioOutputStream {
    played_buffer: Arc<Mutex<Vec<f32>>>,
    config: AudioStreamConfig,
    is_active: bool,
}

impl AudioOutputStream for MockAudioOutputStream {
    fn start(&mut self) -> Result<()> {
        self.is_active = true;
        Ok(())
    }

    fn write_chunk(&mut self, samples: &[f32]) -> Result<()> {
        if self.is_active {
            if let Ok(mut lock) = self.played_buffer.lock() {
                lock.extend_from_slice(samples);
            }
        }
        Ok(())
    }

    fn flush_and_wait(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.is_active = false;
        Ok(())
    }

    fn config(&self) -> AudioStreamConfig {
        self.config
    }
}

impl AudioHal for MockAudioHal {
    fn open_input_stream(
        &self,
        _device_name: Option<&str>,
        config: &AudioStreamConfig,
    ) -> Result<Box<dyn AudioInputStream>> {
        Ok(Box::new(MockAudioInputStream {
            samples: Arc::clone(&self.captured_samples),
            config: *config,
            is_active: false,
        }))
    }

    fn open_output_stream(
        &self,
        _device_name: Option<&str>,
        config: &AudioStreamConfig,
    ) -> Result<Box<dyn AudioOutputStream>> {
        Ok(Box::new(MockAudioOutputStream {
            played_buffer: Arc::clone(&self.played_samples),
            config: *config,
            is_active: false,
        }))
    }
}
