use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;

mod defaults {
    pub(super) const fn cooldown() -> usize {6}
    pub(super) const fn timeout() -> usize {10}
    pub(super) const fn r#true() -> bool {true}
    pub(super) fn database_file() -> String {String::from("giveaways.json")}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MeiliSearchConfig {
    pub host: String,
    pub index: String,
    pub key: String,
    #[serde(default = "defaults::r#true")]
    pub init_on_launch: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "defaults::cooldown")]
    pub cooldown: usize,
    #[serde(default)]
    pub update: usize,
    #[serde(default = "defaults::timeout")]
    pub timeout: usize,
    #[serde(default)]
    pub blame_useless_pages: bool,
    #[serde(default = "defaults::database_file")]
    pub database_file: String,
    pub meilisearch: Option<MeiliSearchConfig>,
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