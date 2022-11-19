use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use webrtc::ice_transport::ice_credential_type::RTCIceCredentialType;
use webrtc::ice_transport::ice_server::RTCIceServer;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_signaller")]
    pub signaller_url: String,

    #[serde(default = "default_viewer")]
    pub viewer_url: String,

    #[serde(default = "default_max_fps")]
    pub max_fps: u32,

    #[serde(default = "default_ice_servers")]
    pub ice_servers: Vec<IceServer>,

    #[serde(default = "libx264")]
    pub encoder: EncoderConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub encoder: String,
    pub pixel_format: String,
    #[serde(serialize_with = "toml::ser::tables_last")]
    pub options: HashMap<String, String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IceCredentialType {
    Unspecified,
    Password,
    Oauth,
}

impl Default for IceCredentialType {
    fn default() -> Self {
        IceCredentialType::Password
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub credential: String,
    #[serde(default)]
    pub credential_type: IceCredentialType,
}

impl Into<RTCIceCredentialType> for IceCredentialType {
    fn into(self) -> RTCIceCredentialType {
        match self {
            IceCredentialType::Unspecified => RTCIceCredentialType::Unspecified,
            IceCredentialType::Password => RTCIceCredentialType::Password,
            IceCredentialType::Oauth => RTCIceCredentialType::Oauth,
        }
    }
}

impl Into<RTCIceServer> for IceServer {
    fn into(self) -> RTCIceServer {
        RTCIceServer {
            urls: self.urls,
            username: self.username,
            credential: self.credential,
            credential_type: self.credential_type.into(),
        }
    }
}

pub fn load(path: &Path) -> Result<Config> {
    // create a new file if it does not exist
    if !path.exists() {
        let mut file = File::create(path)?;
        let config = toml::from_str::<Config>("")?;
        file.write_all(toml::to_string(&config)?.as_ref())?;
        return Ok(config);
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(toml::from_str(&contents)?)
}

fn libx264() -> EncoderConfig {
    EncoderConfig {
        encoder: "libx264".to_string(),
        pixel_format: if cfg!(target_os = "windows") {
            "yuv420p".to_string()
        } else if cfg!(target_os = "macos") {
            "nv12".to_string()
        } else {
            panic!("Unsupported platform");
        },
        options: HashMap::from([
            ("profile".into(), "baseline".into()),
            ("preset".into(), "ultrafast".into()),
            ("tune".into(), "zerolatency".into()),
        ]),
    }
}

fn default_signaller() -> String {
    "wss://ws.mirashare.app".to_string()
}

fn default_viewer() -> String {
    "https://mirashare.app/".to_string()
}

fn default_max_fps() -> u32 {
    120
}

fn default_ice_servers() -> Vec<IceServer> {
    vec![
        IceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        },
        IceServer {
            // TURN server from [Open Relay Project](https://openrelayproject.org)
            urls: vec!["turn:openrelay.metered.ca:80".to_string()],
            username: "openrelayproject".to_string(),
            credential: "openrelayproject".to_string(),
            credential_type: IceCredentialType::Password,
        },
    ]
}
