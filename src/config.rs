use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::to_writer;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_signaller")]
    pub signaller_url: String,

    #[serde(default = "default_viewer")]
    pub viewer_url: String,

    #[serde(default = "default_max_fps")]
    pub max_fps: u32,

    #[serde(default = "libx264")]
    pub encoder: EncoderConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncoderConfig {
    encoder: String,
    yuv_input: bool,
    #[serde(serialize_with = "toml::ser::tables_last")]
    options: HashMap<String, String>,
}

pub fn load(path: &Path) -> Result<Config> {
    // create a new file if it does not exist
    if !path.exists() {
        let mut file = File::create(path)?;
        let config = toml::from_str::<Config>("")?;
        info!("config {:#?}", config);
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
        yuv_input: false,
        options: HashMap::from([
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
