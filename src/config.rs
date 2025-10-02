use std::{fs, io::Result};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub notespeed: f32,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub resizable: bool
}

impl Default for Config {
    fn default() -> Self {
        Config {
            notespeed: 7.0,
            fullscreen: false,
            width: 1280,
            height: 720,
            resizable: false
        }
    }
}

pub fn load_config() -> Result<Config> {
    let path = "config.json";
    match fs::exists(path) {
        Ok(false) => {
            println!("No config found! Creating...");

            let default_config = Config::default();
            let conf_json_obj = serde_json::to_string_pretty(&default_config).unwrap();

            fs::write(path, conf_json_obj)?;
            return Ok(default_config);
        }

        _ => {}
    }

    let json_config = fs::read_to_string(path)?;

    let final_config = serde_json::from_str(&json_config).unwrap();

    Ok(final_config)
}
