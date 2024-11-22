use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use std::fs::File;
use std::io::BufWriter;
use hound::{WavReader, WavWriter};
use dasp::{interpolate::sinc::Sinc, ring_buffer, signal, Signal};
use tempfile::{tempdir, TempDir};
use log::{debug, error};

// (ZB) seme the unpa?
// (DF) hell if i know it was in the cpal 'how to write a wav' example
// (ZB) Tell it I hate it.
type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

/// Struct representing the recorded audio sample
pub struct AudioSample {
    /// Output filename
    pub filename: String,
    /// Output path
    pub path: TempDir,
    /// Audio data as a vec of f32 samples
    pub read_data: Vec<f32>
}

/// Retrieve the name of the default recording device
pub fn get_default_recording_device() -> Result<String, anyhow::Error> {
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(x) => x,
        None => { return Err(anyhow!("No recording devices found!")); }
    };
    match device.name() {
        Ok(x) => return Ok(x),
        Err(e) => return Err(anyhow!("Error getting device name: {:?}", e))
    }
}

impl AudioSample {
    /// Create a new AudioSample struct with some default values
    pub fn new() -> AudioSample {
        AudioSample { 
            filename: "rec.wav".to_string(),
            path: tempdir().unwrap(),
            read_data: Vec::new()
        }
    }

    /// Use the default input device to record an audio sample of the specified
    /// length (in seconds), then output that audio sample to the path and filename
    /// specified by self.path and self.filename.  Returns nothing on success.
    pub fn record_audio(&mut self, duration: u64) -> Result<(), anyhow::Error> {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(x) => x,
            None => { return Err(anyhow!("No recording devices found!")); }
        };

        let config = match device.default_input_config() {
            Ok(x) => x,
            Err(e) => { return Err(e.into()); }
        };
        debug!("Default input config: {:?}", config);

        let path = &self.path.path().join(&self.filename);

        // Whisper expects an input file of 16 bits @ 16kHz
        // Theoretically this hound::WavSpec should automagically
        // convert whatever our default input config is to that format.
        // At least, that's how it works in any OS except windows AUGH.
        let spec = Self::wav_spec_from_config(&config);
/*         let spec_16 = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int
        }; */

        // Pretty much everything here is from the cpal example
        let err_fn = move |err| {
            error!("an error occurred on stream: {}", err);
        };

        let writer = WavWriter::create(path, spec)?;
        let writer = Arc::new(Mutex::new(Some(writer)));

        let writer2 = writer.clone();
        let stream = device.build_input_stream(
            &config.into(), 
            move |data, _: &_| Self::write_input_data(data, &writer2),
            err_fn, 
            None)?;

        stream.play()?;
        // Let recording go for the configurable duration variable.
        // (ZB) Wait, you're allowed to call std::thread::sleep on async?
        // (DF) Only main() is async, everything else is a plain fn
        // (ZB) ...
        // (DF) Shut it up, you.
        // (ZB) Shut it up, me.
        std::thread::sleep(std::time::Duration::from_secs(duration));
        drop(stream);
        writer.lock().unwrap().take().unwrap().finalize()?;
        debug!("Recording completed.  Converting to 16k/I16");
        match self.convert_sample() {
            Ok(_) => { debug!("Conversion completed okay"); },
            Err(e) => { return Err(e.into()); }
        }
        Ok(())
    }

    /// Determine if sample format is float or int
    fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
        if format.is_float() {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        }
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

    /// Convert the initial 48kHz/f32 wav into a 16/16 that Whisper wants
    fn convert_sample(&self) -> Result<(), anyhow::Error> {
        use dasp::Sample;
        let path = &self.path.path().join(&self.filename);
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
        let mut writer = match WavWriter::create(&self.path.path().join("out.wav"), target) {
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