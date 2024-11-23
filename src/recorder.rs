//! STTRecorder
//! This module defines functions for recording audio to a temporary file
//! and ensuring it is in the format required for whisper-rs (16KHz mono f32).
//! Some of this logic is extraneous on Linux and MacOS but since this is ultimately
//! meant to run under Windows, it's written to run under Windows.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use dasp::{interpolate::sinc::Sinc, ring_buffer, signal, Signal};
use log::{debug, error};

/// Struct representing the recorded audio sample
pub struct STTRecorder {
    /// The input device to read audio from
    pub input_device: cpal::Device,
    /// For convenience sake to avoid extraneous 'match input_device.name()' blocks
    pub device_name: String,
    /// Audio data as a vec of f32 samples
    pub audio_data: Vec<f32>
}

impl STTRecorder {
    /// Create a new STTRecorder struct with some default values
    pub fn new() -> Result<STTRecorder, anyhow::Error> {
        // Get the default device info
        let host = cpal::default_host();
        let input_device = match host.default_input_device() {
            Some(x) => x,
            None => { return Err(anyhow!("No recording devices found!")); }
        };
        let device_name = match input_device.name() {
            Ok(x) => x,
            Err(e) => return Err(anyhow!("Error getting device name: {:?}", e))
        };
        let audio_data = Vec::new();
        Ok(STTRecorder { input_device, device_name, audio_data })
    }

    /// Use the default input device to record an audio sample of the specified
    /// length (in seconds)
    pub fn record_audio(&mut self, duration: u64) -> Result<(), anyhow::Error> {
        // Get the default input config for our recording device
        let config = match self.input_device.default_input_config() {
            Ok(x) => {
                debug!("Default input config: {:?}", x);
                x
            },
            Err(e) => {
                error!("Error getting default input config");
                return Err(e.into()); 
            }
        };
        
        // Whisper expects an input file of 16kHz mono f32.
        // CPAL doc says to build a hound::WavSpec using the default config
        let spec = Self::wav_spec_from_config(&config);

        // Much of this is from the cpal example code
        let err_fn = move |err| {
            error!("an error occurred on stream: {}", err);
        };

        let data_buffer = Vec::<f32>::new();
        let arcbuf = Arc::new(Mutex::new(Some(data_buffer)));
        let arcbuf2 = arcbuf.clone();

        // Being lazy and using ? on these.  Create the
        // input stream with a callback to the store-in-memory function
        let stream = self.input_device.build_input_stream(
            &config.into(), 
            move |data, _: &_| Self::store_input_data(data, &arcbuf2),
            err_fn, 
            None)?;

        // Start recording
        stream.play()?;

        // Let recording go for the configurable duration variable.
        std::thread::sleep(std::time::Duration::from_secs(duration));

        // Stop recording and drop the stream
        drop(stream);
        debug!("Recording completed.");

        // Convert the recorded samples to 16kHz/32-bit
        let converted = match Self::convert_samples(&arcbuf, &spec) {
            Ok(x) => {
                debug!("Conversion ok");
                x
            },
            Err(e) => {
                error!("Error converting samples");
                return Err(e.into());
            }
        };

        // Stash the recorded audio in self and return
        self.audio_data = converted;
        Ok(())
    }

    /// Move input data into buffer vec.  We're using this gross 
    /// arc mutex thingy because cloning or something.
    fn store_input_data(data: &[f32], buf: &Arc<Mutex<Option<Vec<f32>>>>) {
        use cpal::Sample;
        if let Ok(mut guard) = buf.try_lock() {
            if let Some(buffer) = guard.as_mut() {
                for &sample in data.iter() {
                    let sample: f32 = f32::from_sample(sample);
                    buffer.push(sample);
                }
            }
        }
    }

    /// Determine if sample format is float or int
    fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
        if format.is_float() { hound::SampleFormat::Float } 
        else { hound::SampleFormat::Int }
    }

    /// Build a hound WavSpec from the supplied cpal config
    fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
        hound::WavSpec {
            channels: config.channels() as _,
            sample_rate: config.sample_rate().0 as _,
            bits_per_sample: (config.sample_format().sample_size() * 8) as _,
            sample_format: Self::sample_format(config.sample_format()),
        }
    }

    /// Take whatever format the recorded samples are in and conver them to
    /// 16kHz/mono/32-bit.  Gonna be honest, I don't know WHAT the heck this is doing.
    fn convert_samples(buf: &Arc<Mutex<Option<Vec<f32>>>>, spec: &hound::WavSpec) -> Result<Vec<f32>, anyhow::Error> {
        use dasp::Sample;
        // Get the vec back out of the big mess up there
        let mut retval: Vec<f32> = Vec::new();
        let mut samples = Vec::new();
        if let Ok(mut guard) = buf.try_lock() {
            if let Some(data) = guard.as_mut() {
                for &sample in data.iter() {
                    samples.push(sample);
                }
            } else { 
                error!("Error extracting samples from Mutex"); 
                return Err(anyhow!("Error in convert_samples on guard_as_mut()"));
            }
        } else { 
            error!("Error extracting samples from Mutex");
            return Err(anyhow!("Error in convert_samples on buf_try_lock()"));
        }
        // I don't even think most of this is necessary just 'sample_rate'
        let mut target = hound::WavSpec::from(*spec);
        target.sample_rate = 16_000;
        target.sample_format = hound::SampleFormat::Float;
        target.bits_per_sample = 32;
        target.channels = 1;
        let signal = signal::from_interleaved_samples_iter(samples.iter().cloned());
        let ring_buffer = ring_buffer::Fixed::from([[0.0]; 100]);
        let sinc = Sinc::new(ring_buffer);
        let new_signal = signal.from_hz_to_hz(sinc, spec.sample_rate as f64, target.sample_rate as f64);
        for frame in new_signal.until_exhausted() {
            retval.push(frame[0].to_sample());
        }
        Ok(retval)
    }
}