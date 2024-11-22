#[allow(unused)]
use hound;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use log::{debug, error};

use crate::cpal::AudioSample;

pub struct WhisperProcessor {
    pub modelpath: String,
    pub outputstring: String
}

impl WhisperProcessor {
    pub fn new(p: String) -> WhisperProcessor {
        WhisperProcessor { modelpath: p, outputstring: String::new() }
    }

    pub fn load_and_process(&mut self, sample: &AudioSample, lang: &str) -> Result<(), anyhow::Error> {
        let ctx = match WhisperContext::new_with_params(&self.modelpath, WhisperContextParameters::default()) {
            Ok(x) => {
                debug!("Created WhisperContext ok");
                x
            },
            Err(e) => { 
                error!("Error creating WhisperContext");
                return Err(e.into()); 
            }
        };
        let mut state = match ctx.create_state() {
            Ok(x) => {
                debug!("Created context state okay");
                x
            },
            Err(e) => {
                error!("Error creating context state");
                return Err(e.into());
            }
        };

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_n_threads(8);
        params.set_translate(false);
        params.set_language(Some(lang));
        params.set_suppress_non_speech_tokens(true);
        params.set_print_progress(false);
        params.set_single_segment(true);

        // (ZB) Why is this hard-coded
        // (DF) Because it's hard-coded in cpal.rs
        // (ZB) Why is it hard-coded there?
        // (DF) Because shut up >:(
        let filename = &sample.path.path().join("rec.wav");
        let reader = match hound::WavReader::open(filename) {
            Ok(x) => {
                debug!("Opened wav file ok");
                x
            },
            Err(e) => { 
                error!("Error opening wav file");
                return Err(e.into()); 
            }
        };

        #[allow(unused_variables)]
        let hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample,
            ..
        } = reader.spec();

        // Convert to float
        // (ZB) Hang on why are we converting BACK to float after all the trouble converting to int?
        // (DF) Whisper wants i16/16kHz in the file I dunno why
        let samples: Vec<i16> = reader
            .into_samples::<i16>()
            .map(|x| x.expect("invalid sample"))
            .collect();
        let mut audio = vec![0.0f32; samples.len().try_into().unwrap()];
        match whisper_rs::convert_integer_to_float_audio(&samples, &mut audio) {
            Ok(_) => { debug!("Successfully converted samples to float");},
            Err(e) => { error!("Error converting samples to float"); return Err(e.into()); }
        }
        
        // Make sure samples are mono
        if channels == 2 {
            audio = match whisper_rs::convert_stereo_to_mono_audio(&audio) {
                Ok(x) => {
                    debug!("Mono downconvert ok");
                    x
                },
                Err(e) => {
                    error!("Error downconverting to mono");
                    return Err(e.into());
                }
            }
        }

        // Make sure sample rate is 16kHz
        // I'll allow a panic here because if it's not 16kHz something is majorly wrong
        if sample_rate != 16000 { panic!("Sample rate not 16khz"); }

        // Run the model
        match state.full(params, &audio[..]) {
            Ok(_) => { debug!("Model ran successfully"); },
            Err(e) => { error!("Error running model"); return Err(e.into()); }
        }

        // Iterate through the segments of the audio
        let num_segments = match state.full_n_segments() {
            Ok(x) => {
                debug!("Got full_n_segments");
                x
            },
            Err(e) => {
                error!("Error getting full_n_segments");
                return Err(e.into());
            }
        };
        for i in 0..num_segments {
            let segment = match state.full_get_segment_text(i) {
                Ok(x) => {
                    debug!("Got STT segment {}: {}", i, x);
                    x
                },
                Err(e) => {
                    error!("Error getting STT segment {}", i);
                    return Err(e.into());
                }
            };
            if i == 0 { self.outputstring = format!("{}", segment); }
        }
        Ok(())
    }
}