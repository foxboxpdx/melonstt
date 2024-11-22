#[allow(unused)]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use std::fs::File;
use std::io::BufWriter;
use hound::WavWriter;
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
        let spec_16 = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int
        };

        // Pretty much everything here is from the cpal example
        let err_fn = move |err| {
            error!("an error occurred on stream: {}", err);
        };

        let writer = WavWriter::create(path, spec_16)?;
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
        debug!("Recording completed.");
        Ok(())
    }

    /// Take the incoming sample(s) and output them to the output file
    /// The samples are assumed to be f32 since that seems to be the default
    /// bits-per-sample.  We can use cpal::Sample's 'from_sample' function
    /// to re-cast them into the i16 we told Hound to expect.
    fn write_input_data(input: &[f32],  writer: &WavWriterHandle) {
        use cpal::Sample;
        if let Ok(mut guard) = writer.try_lock() {
            if let Some(writer) = guard.as_mut() {
                for &sample in input.iter() {
                    let sample: i16 = i16::from_sample(sample);
                    writer.write_sample(sample).ok();
                }
            } else { error!("Error getting guard.as_mut in write_input_data!"); }
        } else { error!("Error getting lock in write_input_data!"); }
    }
}