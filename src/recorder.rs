//! STTRecorder
//! This module defines functions for recording audio to a temporary file
//! and ensuring it is in the format required for whisper-rs (16KHz mono f32).
//! Some of this logic is extraneous on Linux and MacOS but since this is ultimately
//! meant to run under Windows, it's written to run under Windows.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use std::{fs::File, io::BufWriter};
use hound::{WavReader, WavWriter};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use dasp::{interpolate::sinc::Sinc, ring_buffer, signal, Signal};
use tempfile::{tempdir, TempDir};
use log::{debug, error};

// (ZB) seme the unpa?
// (DF) hell if i know it was in the cpal 'how to write a wav' example
// (ZB) Tell it I hate it.
type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

/// Struct representing the recorded audio sample
pub struct STTRecorder {
    /// The input device to read audio from
    pub input_device: cpal::Device,
    /// For convenience sake to avoid extraneous 'match input_device.name()' blocks
    pub device_name: String,
    /// Output filename
    pub output_filename: String,
    /// Output path
    pub output_path: TempDir,
    /// Audio data as a vec of f32 samples
    pub audio_data: Vec<f32>
}

impl STTRecorder {
    /// Create a new STTRecorder struct with some default values
    pub fn new() -> Result<STTRecorder, anyhow::Error> {
        // Get the default device info
        let (input_device, device_name) = match Self::get_default_recording_device() {
            Ok((x,y)) => {
                debug!("Got default input device Ok");
                (x,y)
            },
            Err(e) => {
                error!("Error retrieving default input device");
                return Err(e.into());
            }
        };
        let output_path = match tempdir() {
            Ok(x) => {
                debug!("Initialized temdir");
                x
            },
            Err(e) => {
                error!("Error initializing tempdir");
                return Err(e.into());
            }
        };
        let output_filename = "raw_out.wav".to_string();
        let audio_data = Vec::new();
        Ok(STTRecorder { input_device, device_name, output_filename, output_path, audio_data })
    }

    /// Retrieve the default recording device and its name
    fn get_default_recording_device() -> Result<(cpal::Device, String), anyhow::Error> {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(x) => x,
            None => { return Err(anyhow!("No recording devices found!")); }
        };
        match device.name() {
            Ok(x) => return Ok((device, x)),
            Err(e) => return Err(anyhow!("Error getting device name: {:?}", e))
        }
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
        

        let path = &self.output_path.path().join(&self.output_filename);

        // Whisper expects an input file of 16kHz mono f32.
        // CPAL doc says to build a hound::WavSpec using the default config
        let spec = Self::wav_spec_from_config(&config);

        // If our platform lets us do this without munging up the audio, we can
        // set our desired output format
        let desired_spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float
        };

        // Pretty much everything from here down is from the cpal example code
        let err_fn = move |err| {
            error!("an error occurred on stream: {}", err);
        };

        //let writer = WavWriter::create(path, spec)?;
        //let writer = Arc::new(Mutex::new(Some(writer)));

        //let writer2 = writer.clone();
        let data_buffer = Vec::<f32>::new();
        let arcbuf = Arc::new(Mutex::new(Some(data_buffer)));
        let arcbuf2 = arcbuf.clone();

        let stream = self.input_device.build_input_stream(
            &config.into(), 
            //move |data, _: &_| Self::write_input_data(data, &writer2),
            move |data, _: &_| Self::store_input_data(data, &arcbuf2),
            err_fn, 
            None)?;

        stream.play()?;
        // Let recording go for the configurable duration variable.
        std::thread::sleep(std::time::Duration::from_secs(duration));
        drop(stream);
        //writer.lock().unwrap().take().unwrap().finalize()?;
        debug!("Recording completed.");
        /* match self.convert_sample() {
            Ok(_) => { debug!("Conversion completed okay"); },
            Err(e) => { return Err(e.into()); }
        } */
        // Convert the recorded samples to 16kHz/32-bit/mono
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
        // Stash the recorded audio in self
        self.audio_data = converted;
        Ok(())
    }

    /// Move input data into buffer vec
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

    /// Take the incoming sample(s) and output them to the output file
    /// The samples are assumed to be f32 since that seems to be the default
    /// bits-per-sample.  If we weren't on windows we could re-cast them to I16.
    fn write_input_data(input: &[f32],  writer: &WavWriterHandle) {
        use cpal::Sample;
        if let Ok(mut guard) = writer.try_lock() {
            if let Some(writer) = guard.as_mut() {
                for &sample in input.iter() {
                    let sample: f32 = f32::from_sample(sample);
                    writer.write_sample(sample).ok();
                }
            } else { error!("Error getting guard.as_mut in write_input_data!"); }
        } else { error!("Error getting lock in write_input_data!"); }
    }

    /// Take whatever format the recorded samples are in and conver them to
    /// 16kHz/mono/32-bit.  Gonna be honest, I don't know WHAT the heck this is doing.
    fn convert_samples(buf: &Arc<Mutex<Option<Vec<f32>>>>, spec: &hound::WavSpec) -> Result<Vec<f32>, anyhow::Error> {
        use dasp::Sample;
        // Get the vec back out of the big mess up there
        // Just returning an empty vec on failure here
        // I'll fix it later
        // (ZB) no you wont
        // (DF) shush, you.
        let mut retval: Vec<f32> = Vec::new();
        let mut samples = Vec::new();
        if let Ok(mut guard) = buf.try_lock() {
            if let Some(data) = guard.as_mut() {
                for &sample in data.iter() {
                    samples.push(sample);
                }
            } else { return Ok(retval); }
        } else { return Ok(retval); }
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

    /// Convert the initial 48kHz/f32 wav into a 16/16 that Whisper wants
    fn convert_sample(&self) -> Result<(), anyhow::Error> {
        use dasp::Sample;
        let path = &self.output_path.path().join(&self.output_filename);
        let reader = match WavReader::open(path) {
            Ok(x) => {
                debug!("WavReader opened file ok");
                x
            },
            Err(e) => {
                error!("Error reading file in convert_sample");
                return Err(e.into());
            }
        };
        let spec = reader.spec();
        let mut target = spec;
        target.sample_rate = 16_000;
        target.sample_format = hound::SampleFormat::Int;
        target.bits_per_sample = 16;
        let samples = reader
            .into_samples()
            .filter_map(Result::ok)
            .map(f32::to_sample::<f64>);
        let signal = signal::from_interleaved_samples_iter(samples);
        let ring_buffer = ring_buffer::Fixed::from([[0.0]; 100]);
        let sinc = Sinc::new(ring_buffer);
        let new_signal = 
            signal.from_hz_to_hz(sinc, spec.sample_rate as f64, target.sample_rate as f64);
        let mut writer = match WavWriter::create(&self.output_path.path().join("out.wav"), target) {
            Ok(x) => {
                debug!("Created new output file for converted wav OK");
                x
            },
            Err(e) => {
                error!("Error creating new output file for converted wav");
                return Err(e.into());
            }
        };
        for frame in new_signal.until_exhausted() {
            match writer.write_sample(frame[0].to_sample::<i16>()) {
                Ok(_) => { },
                Err(e) => {
                    error!("Error writing sample for converted wav");
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }
}