//! Hardware Abstraction Layer (HAL) for Windows audio input capture and output playback.

pub mod mock;
pub mod resampler;
pub mod traits;
pub mod types;
pub mod windows_cpal;

pub use mock::MockAudioHal;
pub use resampler::{
    convert_to_device_format, downmix_to_mono, normalize_to_16k_mono, pcm_f32_to_i16,
    pcm_i16_to_f32, resample_linear, upmix,
};
pub use traits::{AudioDeviceManager, AudioHal, AudioInputStream, AudioOutputStream};
pub use types::{AudioDeviceDirection, AudioDeviceInfo, AudioStreamConfig, SampleFormat};
pub use windows_cpal::CpalAudioHal;

use std::sync::Arc;

/// Creates the platform default Audio Hardware Abstraction Layer instance.
///
/// On Windows, returns a [`CpalAudioHal`] backed by the native Windows Audio Session API (WASAPI).
pub fn create_default_hal() -> Arc<dyn AudioHal> {
    Arc::new(CpalAudioHal::new())
}
