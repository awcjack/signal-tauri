//! Voice recording widget using cpal for audio capture and hound for WAV encoding

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Voice recorder that captures audio from the default input device
pub struct VoiceRecorder {
    stream: Option<cpal::Stream>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    start_time: Instant,
}

impl VoiceRecorder {
    /// Start recording from the default input device
    pub fn start() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No input device available".to_string())?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = samples.clone();

        let err_fn = |err: cpal::StreamError| {
            tracing::error!("Audio stream error: {}", err);
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = samples_clone.lock() {
                        buf.extend_from_slice(data);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => {
                let samples_clone2 = samples.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = samples_clone2.lock() {
                            buf.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let samples_clone2 = samples.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = samples_clone2.lock() {
                            buf.extend(
                                data.iter()
                                    .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0),
                            );
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err("Unsupported sample format".to_string()),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start recording: {}", e))?;

        Ok(Self {
            stream: Some(stream),
            samples,
            sample_rate,
            channels,
            start_time: Instant::now(),
        })
    }

    /// Stop recording and write samples to a WAV file at the given path.
    /// Returns the path on success.
    pub fn stop(&mut self, output_path: &PathBuf) -> Result<PathBuf, String> {
        // Drop the stream to stop recording
        self.stream.take();

        let samples = self
            .samples
            .lock()
            .map_err(|e| format!("Failed to lock samples: {}", e))?;

        if samples.is_empty() {
            return Err("No audio data recorded".to_string());
        }

        let spec = hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(output_path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        for &sample in samples.iter() {
            let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(amplitude)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;

        Ok(output_path.clone())
    }

    /// Get the duration of the recording so far
    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get duration in seconds based on the number of samples collected
    pub fn duration_secs(&self) -> u32 {
        self.start_time.elapsed().as_secs() as u32
    }
}

impl Drop for VoiceRecorder {
    fn drop(&mut self) {
        // Ensure stream is dropped to release audio device
        self.stream.take();
    }
}
