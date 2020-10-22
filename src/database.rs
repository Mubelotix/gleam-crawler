use std::{collections::HashMap, fs::File, io::prelude::*};
use format::giveaway::SearchResult;
use crate::config::Config;

pub(crate) fn read_database(giveaways: &mut HashMap<String, SearchResult>, config: &Config) {
    match File::open(&config.database_file) {
        Ok(mut file) => {
            let mut content = String::new();
            file.read_to_string(&mut content).expect("Failed to read database");
            let saved_giveaways = serde_json::from_str::<Vec<SearchResult>>(&content).expect("Failed to parse database");
            for saved_giveaway in saved_giveaways {
                match giveaways.remove(&saved_giveaway.giveaway.campaign.key) {
                    Some(old_giveaway) => {
                        let giveaway = saved_giveaway + old_giveaway;
                        giveaways.insert(giveaway.giveaway.campaign.key.clone(), giveaway);
                    },
                    None => {
                        giveaways.insert(saved_giveaway.giveaway.campaign.key.clone(), saved_giveaway);
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("Can't open save file: {}", e);
        }
    }
}

pub(crate) fn save_database(giveaways: &HashMap<String, SearchResult>, config: &Config) {
    let mut file = File::create(&config.database_file).expect("Can't open database file");
    let data = serde_json::to_string(&giveaways.values().collect::<Vec<&SearchResult>>()).expect("Can't serialize database");
    file.write_all(data.as_bytes()).expect("Can't write data to database");
}