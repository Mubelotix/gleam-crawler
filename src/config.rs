use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct MeiliSearchConfig {
    pub host: String,
    pub index: String,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub cooldown: usize,
    pub update: usize,
    pub timeout: usize,
    pub database_file: String,
    pub meilisearch: MeiliSearchConfig
}

pub fn read_config(path: &str) -> Config {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) => {
            panic!("Failed to open the configuration file: {:?}", e);
        }
    };

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let config: Config = match toml::from_str(&content) {
        Ok(config) => config,
        Err(e) => {
            panic!("Your configuration file is not valid: {}\nYou may want to use the `configurate` command to generate a configuration file.", e);
        }
    };
    config
}