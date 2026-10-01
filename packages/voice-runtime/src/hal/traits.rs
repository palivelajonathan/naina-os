//! Trait definitions for audio hardware device management, input capture, and output playback.

use crate::error::Result;
use crate::hal::types::{AudioDeviceInfo, AudioStreamConfig};
use std::fmt::Debug;

/// Abstraction for enumerating and selecting audio input and output devices.
pub trait AudioDeviceManager: Send + Sync + Debug {
    /// Returns a list of available audio input (microphone) devices.
    fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>>;

    /// Returns a list of available audio output (speaker/headphone) devices.
    fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>>;

    /// Returns the system default audio input device.
    fn default_input_device(&self) -> Result<AudioDeviceInfo>;

    /// Returns the system default audio output device.
    fn default_output_device(&self) -> Result<AudioDeviceInfo>;
}

/// Active audio input capture stream.
pub trait AudioInputStream: Send + Sync {
    /// Starts capturing audio frames from the hardware device.
    fn start(&mut self) -> Result<()>;

    /// Stops capturing audio frames.
    fn stop(&mut self) -> Result<()>;

    /// Drains all captured audio samples (f32 normalized in range [-1.0, 1.0]).
    fn capture_samples(&mut self) -> Result<Vec<f32>>;

    /// Returns the negotiated hardware stream configuration.
    fn config(&self) -> AudioStreamConfig;
}

/// Active audio output playback stream.
pub trait AudioOutputStream: Send + Sync {
    /// Starts the playback stream.
    fn start(&mut self) -> Result<()>;

    /// Writes a chunk of interleaved f32 normalized audio samples to the playback queue.
    fn write_chunk(&mut self, samples: &[f32]) -> Result<()>;

    /// Flushes all pending audio samples and blocks until playback has completed.
    fn flush_and_wait(&mut self) -> Result<()>;

    /// Stops playback and halts the stream.
    fn stop(&mut self) -> Result<()>;

    /// Returns the negotiated hardware stream configuration.
    fn config(&self) -> AudioStreamConfig;
}

/// Unified Audio Hardware Abstraction Layer (HAL) interface.
pub trait AudioHal: AudioDeviceManager + Send + Sync + Debug {
    /// Opens an audio input stream for the given device name (or default if None).
    fn open_input_stream(
        &self,
        device_name: Option<&str>,
        config: &AudioStreamConfig,
    ) -> Result<Box<dyn AudioInputStream>>;

    /// Opens an audio output stream for the given device name (or default if None).
    fn open_output_stream(
        &self,
        device_name: Option<&str>,
        config: &AudioStreamConfig,
    ) -> Result<Box<dyn AudioOutputStream>>;
}
