//! High-performance deterministic audio format normalization, channel conversion, and linear resampling.

use crate::types::AudioBuffer;

/// Converts raw little-endian 16-bit signed PCM byte slice into normalized f32 audio samples in [-1.0, 1.0].
pub fn pcm_i16_to_f32(pcm_bytes: &[u8]) -> Vec<f32> {
    pcm_bytes
        .chunks_exact(2)
        .map(|chunk| {
            let s = i16::from_le_bytes([chunk[0], chunk[1]]);
            s as f32 / 32768.0
        })
        .collect()
}

/// Converts normalized f32 audio samples in [-1.0, 1.0] into little-endian 16-bit signed PCM byte vector.
pub fn pcm_f32_to_i16(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let i = (clamped * 32767.0).round() as i16;
        bytes.extend_from_slice(&i.to_le_bytes());
    }
    bytes
}

/// Downmixes interleaved multi-channel audio samples into single-channel mono audio.
pub fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let ch = channels as usize;
    let frames = interleaved.len() / ch;
    let mut mono = Vec::with_capacity(frames);

    for i in 0..frames {
        let frame_start = i * ch;
        let mut sum = 0.0f32;
        for c in 0..ch {
            sum += interleaved[frame_start + c];
        }
        mono.push(sum / ch as f32);
    }
    mono
}

/// Upmixes single-channel mono audio into multi-channel interleaved audio.
pub fn upmix(mono: &[f32], target_channels: u16) -> Vec<f32> {
    if target_channels <= 1 {
        return mono.to_vec();
    }
    let ch = target_channels as usize;
    let mut interleaved = Vec::with_capacity(mono.len() * ch);
    for &s in mono {
        for _ in 0..ch {
            interleaved.push(s);
        }
    }
    interleaved
}

/// High-speed deterministic linear interpolator for audio sample rate conversion.
///
/// Converts a mono or single-stream `input` buffer from `src_rate` to `dst_rate`.
pub fn resample_linear(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if input.is_empty() || src_rate == dst_rate {
        return input.to_vec();
    }
    if src_rate == 0 || dst_rate == 0 {
        return Vec::new();
    }

    let ratio = src_rate as f64 / dst_rate as f64;
    let out_len = ((input.len() as f64) / ratio).round() as usize;
    let mut output = Vec::with_capacity(out_len);

    for out_idx in 0..out_len {
        let in_pos = out_idx as f64 * ratio;
        let idx0 = in_pos.floor() as usize;
        let frac = (in_pos - idx0 as f64) as f32;

        if idx0 >= input.len() {
            break;
        }

        let s0 = input[idx0];
        let s1 = if idx0 + 1 < input.len() {
            input[idx0 + 1]
        } else {
            s0
        };

        let sample = s0 * (1.0 - frac) + s1 * frac;
        output.push(sample);
    }

    output
}

/// Normalizes arbitrary raw hardware capture samples (interleaved, arbitrary sample rate)
/// into the standard 16 kHz 1-channel mono 16-bit PCM [`AudioBuffer`] required by Whisper STT.
pub fn normalize_to_16k_mono(raw_samples: &[f32], src_rate: u32, src_channels: u16) -> AudioBuffer {
    let mono = downmix_to_mono(raw_samples, src_channels);
    let resampled = resample_linear(&mono, src_rate, 16000);
    let pcm_bytes = pcm_f32_to_i16(&resampled);
    AudioBuffer::new(16000, 1, pcm_bytes)
}

/// Converts a 16 kHz mono [`AudioBuffer`] (from Piper TTS) into the device's native playback
/// format (target sample rate, target channel count) with interleaved f32 samples.
pub fn convert_to_device_format(audio: &AudioBuffer, dst_rate: u32, dst_channels: u16) -> Vec<f32> {
    let raw_f32 = pcm_i16_to_f32(&audio.pcm_data);
    let mono_resampled = resample_linear(&raw_f32, audio.sample_rate, dst_rate);
    upmix(&mono_resampled, dst_channels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_conversion_roundtrip() {
        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let bytes = pcm_f32_to_i16(&samples);
        let recovered = pcm_i16_to_f32(&bytes);

        assert_eq!(samples.len(), recovered.len());
        for (a, b) in samples.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[test]
    fn test_downmix_stereo_to_mono() {
        // Interleaved L, R: [1.0, 0.0, 0.5, 0.5, -1.0, 1.0]
        let stereo = vec![1.0, 0.0, 0.5, 0.5, -1.0, 1.0];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.5).abs() < 1e-6);
        assert!((mono[1] - 0.5).abs() < 1e-6);
        assert!((mono[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_upmix_mono_to_stereo() {
        let mono = vec![0.1, 0.2, 0.3];
        let stereo = upmix(&mono, 2);
        assert_eq!(stereo.len(), 6);
        assert_eq!(stereo, vec![0.1, 0.1, 0.2, 0.2, 0.3, 0.3]);
    }

    #[test]
    fn test_resample_48k_to_16k_ratio_3_to_1() {
        // 480 samples @ 48kHz is 10ms -> should become 160 samples @ 16kHz
        let input: Vec<f32> = (0..480).map(|i| (i as f32).sin()).collect();
        let resampled = resample_linear(&input, 48000, 16000);
        assert_eq!(resampled.len(), 160);
    }

    #[test]
    fn test_resample_16k_to_48k_ratio_1_to_3() {
        // 160 samples @ 16kHz is 10ms -> should become 480 samples @ 48kHz
        let input: Vec<f32> = (0..160).map(|i| (i as f32).cos()).collect();
        let resampled = resample_linear(&input, 16000, 48000);
        assert_eq!(resampled.len(), 480);
    }

    #[test]
    fn test_normalize_to_16k_mono_full_chain() {
        // 48kHz stereo input, 10ms (480 frames * 2 = 960 samples)
        let raw: Vec<f32> = vec![0.25; 960];
        let audio = normalize_to_16k_mono(&raw, 48000, 2);

        assert_eq!(audio.sample_rate, 16000);
        assert_eq!(audio.channels, 1);
        // 160 samples * 2 bytes = 320 bytes
        assert_eq!(audio.pcm_data.len(), 320);
        assert_eq!(audio.duration_ms(), 10);
    }

    #[test]
    fn test_convert_to_device_format_full_chain() {
        // 16kHz mono AudioBuffer, 10ms (160 samples)
        let input_f32 = vec![0.5f32; 160];
        let pcm_bytes = pcm_f32_to_i16(&input_f32);
        let audio = AudioBuffer::new(16000, 1, pcm_bytes);

        // Target: 48kHz stereo
        let output = convert_to_device_format(&audio, 48000, 2);
        // 480 frames * 2 channels = 960 samples
        assert_eq!(output.len(), 960);
        for s in output {
            assert!((s - 0.5).abs() < 0.01);
        }
    }
}
