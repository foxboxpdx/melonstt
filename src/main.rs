use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use crate::cpal::AudioSample;
use crate::whisper::WhisperProcessor;
use std::time::Instant;
use log::{debug, error};
use anyhow::anyhow;

pub mod cpal;
pub mod whisper;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<slint::PlatformError>> {
    // Init EnvLogger
    env_logger::init();

    // Init AppWindow
    let ui = AppWindow::new()?;
    ui.window().on_close_requested(move || { std::process::exit(0); });

    // Create clones of the AppWindow for callback functions
    let ui2 = ui.clone_strong();
    let ui3 = ui.clone_strong();

    // Handle the OSC send button being pressed
    ui.global::<Logic>().on_send_to_osc(move |value| {
            ui3.set_status_text("Sent transcribed text to OSC".into());
            debug!("Sending {} to OSC sender function", &value);
            send_to_osc(&value);
    });

    // Handle a record button being pressed
    ui.global::<Logic>().on_do_recording(move |len| { 
        let length = u64::from_str(&len).unwrap_or(3);
        let now = Instant::now();
        ui2.set_stt_text("RECORDING...".into());
        // Since we can't seem to get any damn feedback in the UI window,
        // just do a regular println here.
        println!("Calling do_recording with length {}", len);
        match do_recording(length) {
            Ok(x) => {
                ui2.set_stt_text(x.into());
                ui2.set_status_text(format!("Processing complete.  Took {:.2?} seconds", now.elapsed()).into());
                debug!("do_recording completed successfully");
            },
            Err(e) => {
                error!("do_recording returned an error: {:?}", e);
                ui2.set_stt_text(e.to_string().into());
                ui2.set_status_text("ERROR!".into());
            }
        };
    });

    ui.set_stt_text("Transcribed text will appear here.  Click button to send to VRC.".into());
    // Get the default input device name
    match cpal::get_default_recording_device() {
        Ok(x) => { 
            debug!("Found default recording device: {}", x);
            ui.set_status_text(format!("Startup OK.  Default recording device: {}.", x).into());
        },
        Err(e) => {
            error!("Error in get_default_recording_device: {:?}", e);
            ui.set_status_text(format!("Startup ERROR finding default recording device!").into());
            ui.set_stt_text(format!("Error: {:?}", e).into());
        }
    };

    let _ = ui.run();
    Ok(())
}

/// Sends the provided text string to the /chatbox/input OSC Endpoint
fn send_to_osc(text: &str) {
    let addr = SocketAddrV4::from_str("127.0.0.1:49001").unwrap();
    let to_addr = SocketAddrV4::from_str("127.0.0.1:9000").unwrap();
    let sock = UdpSocket::bind(addr).unwrap();

    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/chatbox/input".to_string(),
        args: vec![OscType::String(text.to_string()), OscType::Bool(true), OscType::Bool(true) ]
    })).unwrap();

    match sock.send_to(&msg_buf, to_addr) {
        Ok(_) => { debug!("Sent OSC packet successfully"); },
        Err(e) => { error!("Error sending OSC packet: {:?}", e); }
    }
}

/// Toggles the typing indicator via an OSC packet to /chatbox/typing
fn toggle_typing(on: bool) {
    let addr = SocketAddrV4::from_str("127.0.0.1:49001").unwrap();
    let to_addr = SocketAddrV4::from_str("127.0.0.1:9000").unwrap();
    let sock = UdpSocket::bind(addr).unwrap();

    let tog = match on {
        true => OscType::Bool(true),
        false => OscType::Bool(false)
    };

    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/chatbox/typing".to_string(),
        args: vec![tog.clone()]
    })).unwrap();

    match sock.send_to(&msg_buf, to_addr) {
        Ok(_) => { debug!("Sent toggle_typing OSC packet"); },
        Err(e) => { error!("Error sending toggle_typing OSC packet: {:?}", e); }
    }
}

/// Record audio for a specified length of time (in seconds)
/// Output to a temporary wav file, then read and process said file
/// with Whisper to generate transcribed text.  Returns the 
/// transcribed text on success.
fn do_recording(length: u64) -> Result<String, anyhow::Error> {
    toggle_typing(true);
    debug!("Received do-recording callback; toggled typing to on.");
    let mut audio = AudioSample::new();
    match audio.record_audio(length) {
        Ok(_) => { debug!("record_audio() returnd okay"); },
        Err(e) => { return Err(anyhow!("Error while recording: {:?}", e));}
    }
    // (ZB) This should really be dynamic somehow
    // (DF) We'll worry about that in the toki pona version
    // (ZB) ona li pona~
    let mut processor = WhisperProcessor::new("ggml-tiny.en.bin".to_string());
    let retval = match processor.load_and_process(&audio, "en") {
        Ok(_) => { 
            debug!("load_and_process returned ok");
            processor.outputstring.to_string() 
        },
        Err(e) => { return Err(anyhow!("Error processing recording: {:?}", e)); }
    };
    debug!("Record and process ok, toggling typing to off and returning.");
    toggle_typing(false);
    Ok(retval)
}