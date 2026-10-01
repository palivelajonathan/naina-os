//! Data types for audio hardware devices, streams, formats, and configurations.

/// Audio device transmission direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDeviceDirection {
    Input,
    Output,
}

impl std::fmt::Display for AudioDeviceDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioDeviceDirection::Input => write!(f, "Input"),
            AudioDeviceDirection::Output => write!(f, "Output"),
        }
    }
}

/// Metadata and format description of a physical or virtual audio device.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioDeviceInfo {
    /// Unique identifier or internal system identifier for the device.
    pub id: String,
    /// Human-readable device name reported by the operating system.
    pub name: String,
    /// Whether this is the system default device for its direction.
    pub is_default: bool,
    /// Device direction (Input/Microphone vs Output/Speaker).
    pub direction: AudioDeviceDirection,
    /// Default sample rate in Hz configured on the device (e.g. 48000, 44100).
    pub default_sample_rate: u32,
    /// Default channel count (e.g. 1 for mono, 2 for stereo).
    pub default_channels: u16,
    /// Supported sample rates list if reported by backend.
    pub supported_sample_rates: Vec<u32>,
}

/// Audio stream configuration requested or negotiated with an audio device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioStreamConfig {
    /// Desired sample rate in Hz (e.g. 16000 for Whisper, 48000 for Windows speaker).
    pub sample_rate: u32,
    /// Desired channel count (1 for mono, 2 for stereo).
    pub channels: u16,
    /// Frame/buffer size in samples per callback (0 for backend default).
    pub buffer_size: u32,
}

impl Default for AudioStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            buffer_size: 640,
        }
    }
}

/// Audio sample format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    I16,
    U16,
    F32,
}
