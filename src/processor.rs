use hound;
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
        // Should probably theoretically make sure audio_data is f32/16kKz/mono but pfft
        // Run the model
        match self.state.full(self.params.clone(), &audio_data[..]) {
            Ok(_) => { debug!("Model ran successfully"); },
            Err(e) => { error!("Error running model"); return Err(e.into()); }
        }

        // Iterate through the segments of the audio
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
/* 
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
        let filename = &sample.path.path().join("out.wav");
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
    } */
}