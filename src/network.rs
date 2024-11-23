//! This module handles networking functions
//! Primarily sending OSC packets to VRChat via UDP
//! Could be expanded to house a listener as well for inbound OSC data
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use crate::STTConfig;
use log::{debug, error};

/// A struct for holding socket(s) and endpoint(s)
pub struct STTNetwork {
    /// Local side of the UDP socket for OSC
    pub socket: UdpSocket,
    /// Address of remote side of UDP socket for OSC
    pub osc_endpoint: SocketAddrV4,
}

impl STTNetwork {
    /// Create the local socket and parse the endpoint config
    pub fn new(conf: &STTConfig) -> Result<STTNetwork, anyhow::Error> {
        let dest = match &conf.osc_endpoint {
            Some(x) => x,
            None => &"127.0.0.1:9000".to_string()
        };
        let from_addr = match SocketAddrV4::from_str("127.0.0.1:49001") {
            Ok(x) => x,
            Err(e) => { 
                error!("Error creating SocektAddr for local address");
                return Err(e.into());
            }
        };
        let dest_addr = match SocketAddrV4::from_str(&dest) {
            Ok(x) => x,
            Err(e) => {
                error!("Error parsing osc_endpoint address string");
                return Err(e.into())
            }
        };
        let socket = match UdpSocket::bind(from_addr) { 
            Ok(x) => x,
            Err(e) => {
                error!("Error binding local side of UDP socket");
                return Err(e.into());
            }
        };
        Ok(STTNetwork { socket, osc_endpoint: dest_addr })
    }

    /// Sends the provided text string to the /chatbox/input OSC Endpoint
    pub fn send_to_osc(&self, text: &str) -> Result<(), anyhow::Error> {
        // I'm doin this kinda weird because wrapping this in a match is just ugly
        let msg_buf = encoder::encode(
            &OscPacket::Message(
                OscMessage {
                    addr: "/chatbox/input".to_string(),
                    args: vec![OscType::String(text.to_string()), OscType::Bool(true), OscType::Bool(true) ]
                }
            )
        );
        
        let buf = match msg_buf {
            Ok(x) => x,
            Err(e) => {
                error!("Error creating OSC message buffer (send to osc)");
                return Err(e.into());
            }
        };

        match self.socket.send_to(&buf, self.osc_endpoint) {
            Ok(_) => { debug!("Sent OSC packet successfully"); },
            Err(e) => { 
                error!("Error sending OSC packet: {:?}", e); 
                return Err(e.into());
            }
        }
        Ok(())
    }

    /// Toggles the typing indicator via an OSC packet to /chatbox/typing
    pub fn toggle_typing(&self, on: bool) -> Result<(), anyhow::Error> {
        let toggle = match on {
            true => OscType::Bool(true),
            false => OscType::Bool(false)
        };

        let msg_buf = encoder::encode(
            &OscPacket::Message(
                OscMessage {
                    addr: "/chatbox/typing".to_string(),
                    args: vec![toggle.clone()]
                }
            )
        );

        let buf = match msg_buf {
            Ok(x) => x,
            Err(e) => {
                error!("Error creating OSC message buffer (toggle_typing)");
                return Err(e.into());
            }
        };

        match self.socket.send_to(&buf, self.osc_endpoint) {
            Ok(_) => { debug!("Sent toggle_typing OSC packet"); },
            Err(e) => { 
                error!("Error sending toggle_typing OSC packet: {:?}", e); 
                return Err(e.into());
            }
        }
        Ok(())
    }
}