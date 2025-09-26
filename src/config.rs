use std::fs;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub notespeed: f32,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub resizable: bool
}


fn default_config() -> Config {
    Config {
        notespeed: 7.0,
        fullscreen: false,
        width: 1280,
        height: 720,
        resizable: false
    }
}

pub fn load_config() -> Config {

    let path = "config.json";
    let final_config: Config;

    match fs::exists(path) {
        Ok(false) => {
            println!("No config found! Creating...");

            let default_config = default_config();
            fs::File::create(path);

            let conf_json_obj = serde_json::to_string_pretty(&default_config).unwrap();

            fs::write(path, conf_json_obj);

        }

        _ => {}
    }

    let json_config = fs::read_to_string(path)
        .expect("Error");

    final_config = serde_json::from_str(&json_config).unwrap();

    final_config
}
