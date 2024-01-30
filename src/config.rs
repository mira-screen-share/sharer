use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use twilio::TwilioAuthentication;
use webrtc::ice_transport::ice_credential_type::RTCIceCredentialType;
use webrtc::ice_transport::ice_server::RTCIceServer;

use crate::signaller::{Signaller, SignallerIceServer};
use crate::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncoderConfig {
    pub encoder: String,
    pub pixel_format: String,
    pub encoding: String,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IceCredentialType {
    Unspecified,
    #[default]
    Password,
    Oauth,
    Twilio,    // TODO refactor
    Signaller, // TODO refactor
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

impl Into<IceServer> for SignallerIceServer {
    fn into(self) -> IceServer {
        IceServer {
            urls: vec![self.url],
            username: self.username,
            credential: self.password,
            credential_type: IceCredentialType::Password,
        }
    }
}

impl Config {
    async fn fetch_ice_servers_from_signaller(
        &self,
        signaller: Arc<dyn Signaller + Send + Sync>,
    ) -> Vec<IceServer> {
        signaller
            .fetch_ice_servers()
            .await
            .iter()
            .map(|s| s.clone().into())
            .collect()
    }

    pub async fn fetch_ice_servers(&self, signaller: Arc<dyn Signaller + Send + Sync>) -> Self {
        Self {
            ice_servers: futures_util::future::join_all(self.ice_servers.clone().into_iter().map(
                |s| async {
                    match s.credential_type {
                        IceCredentialType::Twilio => get_twilio_ice_servers(s).await,
                        IceCredentialType::Signaller => {
                            self.fetch_ice_servers_from_signaller(signaller.clone())
                                .await
                        }
                        _ => vec![s],
                    }
                },
            ))
            .await
            .iter()
            .flatten()
            .map(|s| s.clone().into())
            .collect(),
            ..self.clone()
        }
    }
}

impl From<IceCredentialType> for RTCIceCredentialType {
    fn from(t: IceCredentialType) -> Self {
        match t {
            IceCredentialType::Unspecified => RTCIceCredentialType::Unspecified,
            IceCredentialType::Password => RTCIceCredentialType::Password,
            IceCredentialType::Oauth => RTCIceCredentialType::Oauth,
            IceCredentialType::Twilio => RTCIceCredentialType::Password,
            IceCredentialType::Signaller => RTCIceCredentialType::Password,
        }
    }
}

impl From<IceServer> for RTCIceServer {
    fn from(server: IceServer) -> RTCIceServer {
        RTCIceServer {
            urls: server.urls,
            username: server.username,
            credential: server.credential,
            credential_type: server.credential_type.into(),
        }
    }
}

pub fn load(path: &Path) -> Result<Config> {
    // create a new file if it does not exist
    if !path.exists() {
        let mut file = File::create(path)?;
        let config = toml::from_str::<Config>("")?;
        file.write_all("# for more sample configs, see https://github.com/mira-screen-share/sharer/tree/main/configs\n".as_bytes())?;
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
        pixel_format: "nv12".to_string(),
        encoding: "video/H264".to_string(),
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
    60
}

fn default_ice_servers() -> Vec<IceServer> {
    vec![
        IceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        },
        IceServer {
            credential_type: IceCredentialType::Signaller,
            ..Default::default()
        },
    ]
}

async fn get_twilio_ice_servers(s: IceServer) -> Vec<IceServer> {
    if s.credential_type != IceCredentialType::Twilio {
        return vec![];
    }
    let base64_engine = base64::engine::GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        base64::engine::general_purpose::PAD,
    );
    let client = twilio::TwilioClient::new(
        "https://api.twilio.com",
        TwilioAuthentication::BasicAuth {
            basic_auth: base64_engine.encode(format!("{}:{}", s.username, s.credential).as_bytes()),
        },
    );
    let response = client.create_token(s.username.as_str()).send().await;
    match response {
        Ok(token) => token
            .ice_servers
            .unwrap_or_default()
            .iter()
            .map(|s| match s {
                Value::Object(s) => {
                    let url = s.get("url").unwrap().as_str().unwrap().to_owned();
                    IceServer {
                        urls: vec![url],
                        username: token.username.clone().unwrap(),
                        credential: token.password.clone().unwrap(),
                        credential_type: IceCredentialType::Password,
                    }
                }
                _ => panic!("Expected object"),
            })
            .collect(),
        Err(e) => {
            error!("Failed to get Twilio ICE servers: {:?}", e);
            vec![]
        }
    }
}
