#![warn(missing_docs)]
//! melonstt - Speech-to-text-to-OSC
//! This file defines a 'melonstt' struct and associated functions
//! to record, process, transcribe, and send spoken speech
//! which can be wrapped in a UI of your choosing
use recorder::STTRecorder;
use processor::STTProcessor;
use log::{debug, error};
use network::STTNetwork;
use serde_derive::Deserialize;

/// This module defines a struct and associated functions for
/// recording and converting incoming speech audio
pub mod recorder;

/// This module defines a struct and associated functions for
/// processing an audio file with Whisper-rs
pub mod processor;

/// This module handles sending OSC packets to VRChat via UDP
pub mod network;

/// A struct representing a configuration file
#[derive(Deserialize)]
pub struct STTConfig {
    /// The language of the incoming speech audio
    pub language: String,
    /// Filename of the whisper language model to use for transcription
    pub model: String,
    /// Optional ip:port for the OSC endpoint in case for some godforsaken
    /// reason this isn't being run on the same machine VRChat is
    pub osc_endpoint: Option<String>,
}

/// Define the struct that does all the things
pub struct MelonSTT {
    /// For recording audio
    pub recorder: STTRecorder,
    /// For processing audio
    pub processor: STTProcessor<'static>,
    /// For sending OSC packets
    pub network: STTNetwork,
}

impl MelonSTT {
    /// Given a config file name, build a new MelonSTT
    pub fn new(config: &str) -> Result<MelonSTT, anyhow::Error> {
        let config = match Self::read_config(config) {
            Ok(x) => {
                debug!("Read config ok");
                x
            },
            Err(e) => {
                error!("Error reading config");
                return Err(e.into());
            }
        };
        let recorder = match STTRecorder::new() {
            Ok(x) => x,
            Err(e) => {
                error!("Error creating STTRecorder");
                return Err(e.into());
            }
        };
        // I cannot for the life of me get past the borrow checker if I try 
        // sending the language along here, so it's getting hard-coded to Some(en)
        // in STTProcessor::new().
        let processor = match STTProcessor::new(config.model.to_string()) {
            Ok(x) => x,
            Err(e) => {
                error!("Error creating STTProcessor");
                return Err(e.into());
            }
        };
        // Prep the networking side
        let network = match STTNetwork::new(&config) {
            Ok(x) => {
                debug!("Initialized networking ok");
                x
            },
            Err(e) => {
                error!("Error initializing networking");
                return Err(e.into());
            }
        };
        Ok(MelonSTT { recorder, processor, network })
    }

    /// Read and process the config file specified by the incoming str
    fn read_config(config: &str) -> Result<STTConfig, anyhow::Error> {
        let conf_data = match std::fs::read_to_string(config) {
            Ok(x) => x,
            Err(e) => {
                error!("Unable to read config file");
                return Err(e.into());
            }
        };
        match toml::from_str(&conf_data) {
            Ok(x) => Ok(x),
            Err(e) => {
                error!("Error parsing config toml");
                return Err(e.into());
            }
        }
    }

    /// Record audio for the specified number of seconds, then process it
    /// and hand back the transcribed string.
    pub fn do_recording(&mut self, seconds: u64) -> Result<String, anyhow::Error> {
        // Toggle the typing indicator on before starting to record
        // It might be overkill to return Err if it fails but it's more
        // likely than not if toggle fails then send will fail.
        match self.network.toggle_typing(true) {
            Ok(_) => { debug!("Toggled typing indicator on"); },
            Err(e) => {
                error!("Error toggling typing indicator on");
                return Err(e.into());
            }
        }
        // Start recording
        match self.recorder.record_audio(seconds) {
            Ok(_) => {
                debug!("Call to do_recording succeeded");
            },
            Err(e) => {
                error!("Error calling do_recording");
                return Err(e.into());
            }
        }
        // Toggle typing indicator back off.
        match self.network.toggle_typing(false) {
            Ok(_) => { debug!("Toggled typing indicator off"); },
            Err(e) => {
                error!("Error toggling typing indicator off");
                return Err(e.into());
            }
        }
        // Process the recorded audio in self.recorder.audio_data
        match self.processor.process(&self.recorder.audio_data) {
            Ok(_) => {
                debug!("Call to process_audio succeeded");
            },
            Err(e) => {
                error!("Error calling process_audio");
                return Err(e.into());
            }
        }
        Ok(self.processor.processed_text.to_string())
    }

    /// Process the recorded audio in self.recorder.audio_data
    pub fn process_audio(&mut self) -> Result<(), anyhow::Error> {
        match self.processor.process(&self.recorder.audio_data) {
            Ok(_) => {
                debug!("Call to process_audio succeeded");
            },
            Err(e) => {
                error!("Error calling process_audio");
                return Err(e.into());
            }
        }
        Ok(())
    }

    /// Send data to VRChat via OSC
    pub fn send_to_osc(&self, data: &str) -> Result<(), anyhow::Error> { 
        match self.network.send_to_osc(data) {
            Ok(_) => {
                debug!("Call to send_to_osc succeeded");
            },
            Err(e) => {
                error!("Error calling send_to_osc");
                return Err(e.into());
            }
        }
        Ok(())
    }
}