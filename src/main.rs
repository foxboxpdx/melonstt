use std::str::FromStr;
use std::time::Instant;
use log::{debug, error};
use melonstt::MelonSTT;
use std::sync::Mutex;
use lazy_static::lazy_static;

slint::include_modules!();

lazy_static! {
    static ref MELON: Mutex<MelonSTT> = {
        let melon = MelonSTT::new("melon.toml").unwrap();
        Mutex::new(melon)
    };
}

fn main() -> Result<(), anyhow::Error> {
    // Init EnvLogger
    env_logger::init();

    // Init MelonSTT struct using specified config filename
/*     let mut melon = match MelonSTT::new("melon.toml") {
        Ok(x) => {
            debug!("initialized melonstt ok");
            x
        },
        Err(e) => {
            error!("Error initializing melonstt");
            return Err(e.into());
        }
    };
    let device_name = melon.recorder.device_name.to_string();
 */
    // Init AppWindow
    let ui = AppWindow::new()?;
    ui.window().on_close_requested(move || { std::process::exit(0); });

    // Create clones of the AppWindow for callback functions
    let ui2 = ui.clone_strong();
    let ui3 = ui.clone_strong();

    // Handle the OSC send button being pressed
    ui.global::<Logic>().on_send_to_osc(move |value| {
            debug!("Sending {} to OSC sender function", &value);
            match MELON.lock().unwrap().send_to_osc(&value) {
                Ok(_) => {
                    ui3.set_status_text("Sent transcribed text to OSC".into());
                },
                Err(e) => {
                    ui3.set_status_text(format!("Error sending to OSC: {:?}", e).into());
                }
            }
    });

    // Handle a record button being pressed
    ui.global::<Logic>().on_do_recording(move |len| { 
        let length = u64::from_str(&len).unwrap_or(3);
        let now = Instant::now();
        ui2.set_stt_text("RECORDING...".into());
        println!("Calling do_recording with length {}", len);
        match MELON.lock().unwrap().do_recording(length) {
            Ok(transcription) => {
                ui2.set_stt_text(transcription.into());
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

    // Set the startup values of the STT Text and Status fields
    ui.set_stt_text("Transcribed text will appear here.  Click button to send to VRC.".into());
    ui.set_status_text(format!("Startup OK.  Input device: {}.", MELON.lock().unwrap().recorder.device_name.to_string()).into());

    // Start up the Slint UI
    let _ = ui.run();
    Ok(())
}