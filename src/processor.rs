//! This module handles processing recorded audio samples through
//! Whisper to transcribe text.
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperState, WhisperContextParameters};
use log::{debug, error};

/// Define a struct to hold our Whisper processing junk
pub struct STTProcessor<'a> {
    /// The Whisper object that does the transcription work
    pub state: WhisperState,
    /// Parameters to pass to the state whenever it's working
    pub params: FullParams<'a, 'a>,
    /// The results of transcription
    pub processed_text: String
}

impl STTProcessor<'_> {
    /// Initialize a new STTProcessor using the given language model
    pub fn new<'a>(model: String) -> Result<STTProcessor<'a>, anyhow::Error> {
        let context = match WhisperContext::new_with_params(&model, WhisperContextParameters::default()) {
            Ok(x) => {
                debug!("Created WhisperContext ok");
                x
            },
            Err(e) => {
                error!("Error creating WhisperContext");
                return Err(e.into());
            }
        };
        let state = match context.create_state() {
            Ok(x) => {
                debug!("Created WhisperState ok");
                x
            },
            Err(e) => {
                error!("Error creating WhisperState");
                return Err(e.into());
            }
        };
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(8);
        params.set_translate(false);
        params.set_language(Some("en"));
        params.set_suppress_non_speech_tokens(true);
        params.set_print_progress(false);
        params.set_single_segment(true);
        Ok(STTProcessor { state, params, processed_text: String::new() })
    }

    /// Process the audio data recorded by STTRecorder
    /// The audio_data should be 32-bit float 16kHz; pretty sure it'll be mono?
    pub fn process(&mut self, audio_data: &[f32]) -> Result<(), anyhow::Error> {
        match self.state.full(self.params.clone(), &audio_data[..]) {
            Ok(_) => { debug!("Model ran successfully"); },
            Err(e) => { error!("Error running model"); return Err(e.into()); }
        }

        // Iterate through the results and store in self.processed_text
        let num_segments = match self.state.full_n_segments() {
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
            let segment = match self.state.full_get_segment_text(i) {
                Ok(x) => {
                    debug!("Got STT segment {}: {}", i, x);
                    x
                },
                Err(e) => {
                    error!("Error getting STT segment {}", i);
                    return Err(e.into());
                }
            };
            if i == 0 { self.processed_text = format!("{}", segment); }
        }
        Ok(())
    }
}