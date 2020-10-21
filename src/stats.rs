use crate::config::*;
use format::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use serde_json as json;
use std::time::SystemTime;
use std::process::exit;

pub fn stats(config: Config) {
    let mut file = match File::open(&config.database_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open the file {}: {}", config.database_file, e);
            exit(1);
        }
    };
    
    let mut content = String::new();
    if let Err(e) = file.read_to_string(&mut content) {
        eprintln!("Failed to read the file {}: {}", config.database_file, e);
        exit(1);
    }

    let giveaways: Vec<SearchResult> = json::from_str(&content).unwrap();
    let total = giveaways.len();
    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let running_giveaways: Vec<&SearchResult> = giveaways.iter().filter(|g| g.ends_at() > timestamp).collect();
            
    println!("running: \t{}", running_giveaways.len());
    println!("ended: \t\t{}", total - running_giveaways.len());
    println!("total: \t\t{}", total);
}