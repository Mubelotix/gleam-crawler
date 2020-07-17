use url::Host;
use gleam_finder::*;
use std::thread;
use std::time::{Duration, SystemTime};
use std::env;
use progress_bar::progress_bar::ProgressBar;
use progress_bar::color::{Color, Style};
use clap::*;
use url::Url;
use std::collections::HashMap;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use serde_json::{to_string, from_str};
use std::fs::File;
use std::io::prelude::*;
use tokio::prelude::*;

fn fix_str_size(mut input: String, size: usize) -> String {
    match input.chars().count() {
        count if count < size => {
            while input.chars().count() < size {
                input.push(' ');
            }
            input
        },
        count if count > size => {
            let mut new_value = String::new();
            for character in input.chars() {
                if new_value.chars().count() < size - 3 {
                    new_value.push(character)
                }
            }

            new_value.push('.');
            new_value.push('.');
            new_value.push('.');
            
            new_value
        },
        _ => {
            input
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    url: String,
    referers: Vec<String>
}

struct IntermediaryUrl {
    url: String,
    domain: Option<Url>
}

impl IntermediaryUrl {
    fn new_from_vec(urls: Vec<String>) -> Vec<Self> {
        let mut result: Vec<Self> = Vec::new();
        for url in urls {
            result.push(IntermediaryUrl::new(url));
        }
        result
    }

    fn new(url: String) -> Self {
        let mut result = Self {
            url,
            domain: None,
        };
        result.init();
        result
    }

    fn init(&mut self) {
        if let Ok(domain) = Url::parse(&self.url) {
            self.domain = Some(domain)
        } else {
            self.domain = None
        }
    }

    fn get_host(&self) -> Host<&str> {
        if let Some(domain) = self.domain.as_ref() {
            if let Some(host) = domain.host() {
                return host;
            }
        }
        Host::Domain("undefined")
    }

    fn get_url(&self) -> &str {
        &self.url
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Giveaway {
    #[serde(flatten)]
    g: gleam_finder::gleam::Giveaway
}

impl meilisearch_sdk::document::Document for Giveaway {
    type UIDType = String;

    fn get_uid(&self) -> &String {
        &self.g.gleam_id
    }
}

impl From<gleam_finder::gleam::Giveaway> for Giveaway {
    fn from(g: gleam_finder::gleam::Giveaway) -> Giveaway {
        Giveaway {
            g
        }
    }
}

#[tokio::main]
async fn main() {
    let matches = App::new("Gleam finder")
        .version("3.1")
        .author("Mubelotix <mubelotix@gmail.com>")
        .about("Search for gleam.io links on the web.")
        .arg(
            Arg::with_name("minimal")
                .long("minimal")
                .short("m")
                .help("Enables simplified mode: only results urls are printed; no progress bar and log informations")
        )
        .arg(
            Arg::with_name("force-cooldown")
                .long("force-cooldown")
                .short("f")
                .help("Force to sleep between every request, even between two differents website.")
        )
        .arg(
            Arg::with_name("save")
                .long("save")
                .short("s")
                .requires("advanced")
                .help("Update the file giveaways.json with new values and delete invalid old giveaways.")
        )
        .arg(
            Arg::with_name("advanced")
                .long("advanced")
                .short("a")
                .help("Scan gleam.io to get informations like number of entries, name and description of the giveaway.")
        )
        .arg(
            Arg::with_name("meili")
                .long("meili")
                .requires("save")
                .requires("meili-host")
                .requires("meili-index")
                .requires("meili-key")
                .help("Enable auto-update of a MeiliSearch document.")
        )
        .arg(
            Arg::with_name("meili-host")
                .long("meili-host")
                .requires("meili")
                .takes_value(true)
                .help("Set the host of the MeiliSearch server. Default: http://localhost:7700")
        )
        .arg(
            Arg::with_name("meili-index")
                .long("meili-index")
                .takes_value(true)
                .requires("meili")
                .help("Set the name of the MeiliSearch index.")
        )
        .arg(
            Arg::with_name("meili-key")
                .long("meili-key")
                .takes_value(true)
                .requires("meili")
                .help("The private key of the MeiliSearch database.")
        )
        .arg(
            Arg::with_name("loop")
                .long("loop")
                .short("l")
                .help("Launch a scan every hour.")
        )
        .arg(
            Arg::with_name("cooldown")
                .long("cooldown")
                .takes_value(true)
                .min_values(0)
                .max_values(86400)
                .default_value("6")
                .help("The in seconds to wait between two requests to the same domain.")
        )
        .arg(
            Arg::with_name("timeout")
                .long("timeout")
                .takes_value(true)
                .min_values(0)
                .max_values(3600)
                .default_value("7")
                .help("Set the timeout for a request.")
        )
        .subcommand(SubCommand::with_name("count")
            .about("Display the number of giveaways saved in the file giveaways.json."))
        .get_matches();
    
    if let Some(_matches) = matches.subcommand_matches("count") {
        if let Ok(mut file) = File::open("giveaways.json") {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                let giveaways: Vec<Giveaway> = from_str(&content).unwrap();
                let total = giveaways.len();
                
                let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                let running_giveaways: Vec<&Giveaway> = giveaways.iter().filter(|g| g.g.end_date > timestamp).collect();
                
                println!("running: \t{}", running_giveaways.len());
                println!("ended: \t\t{}", total - running_giveaways.len());
                println!("total: \t\t{}", total);
                std::process::exit(0);
            } else {
                println!("Can't read giveaways.json.");
                std::process::exit(1);
            }
        } else {
            println!("Can't open giveaways.json.");
            std::process::exit(1);
        }
    }

    let minimal: bool = matches.occurrences_of("minimal") > 0;
    let force_cooldown: bool = matches.occurrences_of("force-cooldown") > 0;
    let save: bool = matches.occurrences_of("save") > 0;
    let auto_enter: bool = matches.occurrences_of("auto-enter") > 0;
    let advanced: bool = matches.occurrences_of("advanced") > 0 || save || auto_enter;
    let loop_enabled: bool = matches.occurrences_of("loop") > 0;
    let meili_update: bool = matches.occurrences_of("meili") > 0;

    let cooldown: u64 = matches.value_of("cooldown").unwrap_or("6").parse().unwrap_or(6);
    env::set_var("MINREQ_TIMEOUT", matches.value_of("timeout").unwrap_or("6"));
    let meili_host: &str = matches.value_of("meili-host").unwrap_or("http://localhost:7700");
    let meili_index: &str = matches.value_of("meili-index").unwrap_or("giveaways");
    let meili_key: &str = matches.value_of("meili-key").unwrap_or("");

    loop {
        let mut giveaways: HashMap<String, Giveaway> = HashMap::new();
        let start = Instant::now();

        if !minimal {
            let mut progress_bar = ProgressBar::new(7);
            progress_bar.set_action("Searching", Color::White, Style::Normal);
    
            let mut results = Vec::new();
            let mut page = 0;
            loop {
                progress_bar.set_action("Loading", Color::Blue, Style::Normal);
                progress_bar.print_info("Getting", &format!("the results page {}", page), Color::Blue, Style::Normal);
                let new_results = google::search(page).unwrap_or_default();
                if !new_results.is_empty() {
                    results.append(&mut IntermediaryUrl::new_from_vec(new_results));
                    page += 1;
                    progress_bar.inc();
                    progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal);
                    thread::sleep(Duration::from_secs(cooldown));
                } else {
                    break;
                }
            }
    
            let mut progress_bar = ProgressBar::new(results.len());
            let mut timeout_check = HashMap::new();
            let mut last_gleam_request = Instant::now();
            progress_bar.set_action("Loading", Color::White, Style::Normal);
            for link_idx in 0..results.len() {
                // verifying if the cooldown is respected
                if force_cooldown {
                    progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal); 
                    thread::sleep(Duration::from_secs(cooldown));
                } else if let Some(last_load_time) = timeout_check.get(&results[link_idx].get_host()) {
                    let time_since_last_load = Instant::now() - *last_load_time;
                    if time_since_last_load < Duration::from_secs(cooldown) {
                        let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                        progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal); 
                        thread::sleep(time_to_sleep);
                    }
                }
                
                progress_bar.set_action("Loading", Color::Blue, Style::Normal);
                for gleam_link in intermediary::resolve(results[link_idx].get_url()).unwrap_or_default() {
                    if advanced {
                        if force_cooldown {
                            progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal);
                            thread::sleep(Duration::from_secs(cooldown));
                        } else {
                            let time_since_last_load = Instant::now() - last_gleam_request;
                            if time_since_last_load < Duration::from_secs(cooldown) {
                                let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                                progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal);
                                thread::sleep(time_to_sleep);
                            }
                        }
    
                        progress_bar.set_action("Loading", Color::Blue, Style::Normal);
                        if let Ok(giveaway) = gleam_finder::gleam::Giveaway::fetch(&gleam_link) {
                            last_gleam_request = Instant::now();
                            progress_bar.print_info("Found", &format!("{} {:>8} entries - {}", giveaway.get_url(), if let Some(entry_count) = giveaway.entry_count { entry_count.to_string() } else {String::from("unknow")}, giveaway.name), Color::LightGreen, Style::Bold);
                            giveaways.insert(gleam_link, giveaway.into());
                        }
                    } else {
                        progress_bar.print_info("Found", &gleam_link, Color::LightGreen, Style::Bold);
                    }
                }
                
                progress_bar.inc();
                timeout_check.insert(results[link_idx].get_host(), Instant::now());
            }
        } else {
            let mut results = Vec::new();
            let mut page = 0;
            loop {
                let new_results = google::search(page).unwrap_or_default();
                if !new_results.is_empty() {
                    results.append(&mut IntermediaryUrl::new_from_vec(new_results));
                    page += 1;
                    thread::sleep(Duration::from_secs(cooldown));
                } else {
                    break;
                }
            }
    
            let mut timeout_check = HashMap::new();
            let mut last_gleam_request = Instant::now();
    
            for link_idx in 0..results.len() {
                // verifying if the cooldown is respected
                if force_cooldown {
                    thread::sleep(Duration::from_secs(cooldown));
                } else if let Some(last_load_time) = timeout_check.get(&results[link_idx].get_host()) {
                    let time_since_last_load = Instant::now() - *last_load_time;
                    if time_since_last_load < Duration::from_secs(cooldown) {
                        let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                        thread::sleep(time_to_sleep);
                    }
                }
                
                for gleam_link in intermediary::resolve(results[link_idx].get_url()).unwrap_or_default() {
                    println!("{}", gleam_link);
                    if advanced {
                        if force_cooldown {
                            thread::sleep(Duration::from_secs(cooldown));
                        } else {
                            let time_since_last_load = Instant::now() - last_gleam_request;
                            if time_since_last_load < Duration::from_secs(cooldown) {
                                let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                                thread::sleep(time_to_sleep);
                            }
                        }
    
                        if let Ok(giveaway) = gleam_finder::gleam::Giveaway::fetch(&gleam_link) {
                            last_gleam_request = Instant::now();
                            giveaways.insert(gleam_link, giveaway.into());
                        }
                    }
                }
                timeout_check.insert(results[link_idx].get_host(), Instant::now());
            }
        }
        
        if save {
            match File::open("giveaways.json") {
                Ok(mut file) => {
                    let mut content = String::new();
                    match file.read_to_string(&mut content) {
                        Ok(_) => match from_str::<Vec<Giveaway>>(&content) {
                            Ok(saved_giveaways) => for saved_giveaway in saved_giveaways {
                                if giveaways.get(&saved_giveaway.g.gleam_id).is_none() {
                                    giveaways.insert(saved_giveaway.g.get_url().to_string(), saved_giveaway);
                                }
                            },
                            Err(e) => eprintln!("Can't deserialize save file: {}", e)
                        }
                        Err(e) => eprintln!("Can't read save file: {}", e)
                    }
                },
                Err(e) => eprintln!("Can't open save file: {}", e)
            }

            match File::create("giveaways.json") {
                Ok(mut file) => {
                    match to_string(&giveaways.values().collect::<Vec<&Giveaway>>()) {
                        Ok(data) => match file.write(data.as_bytes()) {
                            Ok(_) => (),
                            Err(e) => eprintln!("Can't write to file: {}", e)
                        },
                        Err(e) => eprintln!("Can't serialize data: {}", e)
                    }
                }
                Err(e) => eprintln!("Can't open save file: {}", e)
            }

            if meili_update {
                let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                let giveaways = giveaways.drain().map(|(_i, g)| g).filter(|g| g.g.end_date > timestamp).collect::<Vec<Giveaway>>();
                
                use meilisearch_sdk::client::Client;
                async fn update_database(meili_host: &str, meili_key: &str, meili_index: &str, running_giveaways: &Vec<Giveaway>) {
                    let client = Client::new(meili_host, meili_key);
                    let index = match client.get_or_create(meili_index).await {
                        Ok(index) => index,
                        Err(e) => {
                            eprintln!("Meilisearch error while initializing the index: {:?}", e);
                            return;
                        },
                    };
                    if let Err(e) = index.delete_all_documents().await {
                        eprintln!("Meilisearch error while deleting documents: {:?}", e);
                        return;
                    };
                    if let Err(e) = index.add_documents(running_giveaways, Some("gleam_id")).await {
                        eprintln!("Meilisearch error while adding documents: {:?}", e);
                        return;
                    };
                }
                
                update_database(meili_host, meili_key, meili_index, &giveaways).await
            }
        }

        if loop_enabled {
            let time_elapsed = Instant::now().duration_since(start);
            let time_to_sleep = Duration::from_secs(3600) - time_elapsed;
            thread::sleep(time_to_sleep);
        } else {
            break;
        }
    }
}