use crate::{config::*, crawler_lib::*};
use std::{collections::HashMap, time::{Instant, Duration, SystemTime}, thread::sleep};
use progress_bar::{color::*, progress_bar::ProgressBar};
use serde::{Serialize, Deserialize};
use url::{Url, Host};
use format::prelude::*;

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

pub async fn launch(config: Config, fast: bool) {
    std::env::set_var("MINREQ_TIMEOUT", config.timeout.to_string());
    let cooldown = config.cooldown as u64;

    loop {
        let mut giveaways: HashMap<String, SearchResult> = HashMap::new();
        let start = Instant::now();

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
                sleep(Duration::from_secs(cooldown));
            } else {
                break;
            }
        }
        progress_bar.set_action("Finished", Color::Green, Style::Bold);
        progress_bar.print_info("Finished", &format!("{} results found", results.len()), Color::Green, Style::Bold);
        progress_bar.finalize();

        let mut progress_bar = ProgressBar::new(results.len());
        let mut timeout_check = HashMap::new();
        let mut last_gleam_request = Instant::now();
        progress_bar.set_action("Loading", Color::White, Style::Normal);
        for link_idx in 0..results.len() {
            // verifying if the cooldown is respected
            if let Some(last_load_time) = timeout_check.get(&results[link_idx].get_host()) {
                let time_since_last_load = Instant::now() - *last_load_time;
                if time_since_last_load < Duration::from_secs(cooldown) {
                    let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                    progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal); 
                    sleep(time_to_sleep);
                }
            }
            
            progress_bar.set_action("Loading", Color::Blue, Style::Normal);
            let giveaway_urls = intermediary::resolve(results[link_idx].get_url()).unwrap_or_default();
            if giveaway_urls.is_empty() {
                progress_bar.print_info("Useless", &format!("page loaded: {}", results[link_idx].get_url()), Color::Yellow, Style::Normal);
            }
            for gleam_link in giveaway_urls {
                if !fast {
                    let time_since_last_load = Instant::now() - last_gleam_request;
                    if time_since_last_load < Duration::from_secs(cooldown) {
                        let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                        progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal);
                        sleep(time_to_sleep);
                    }

                    progress_bar.set_action("Loading", Color::Blue, Style::Normal);
                    if let Ok(giveaway) = crate::crawler_lib::gleam::fetch(&gleam_link) {
                        last_gleam_request = Instant::now();
                        progress_bar.print_info("Found", &format!("{} {:>8} entries - {}", giveaway.get_url(), if let Some(entry_count) = giveaway.entry_count { entry_count.to_string() } else {String::from("unknow")}, giveaway.get_name()), Color::LightGreen, Style::Bold);
                        giveaways.insert(gleam_link, giveaway);
                    }
                } else {
                    progress_bar.print_info("Found", &gleam_link, Color::LightGreen, Style::Bold);
                }
            }
            
            progress_bar.inc();
            timeout_check.insert(results[link_idx].get_host(), Instant::now());
        }
        progress_bar.set_action("Finished", Color::Green, Style::Bold);
        progress_bar.print_info("Finished", &format!("{} giveaways found", giveaways.len()), Color::Green, Style::Bold);
        progress_bar.finalize();
        println!();
        
        if !fast {
            use std::fs::File;
            use std::io::prelude::*;

            match File::open("giveaways.json") {
                Ok(mut file) => {
                    let mut content = String::new();
                    match file.read_to_string(&mut content) {
                        Ok(_) => match serde_json::from_str::<Vec<SearchResult>>(&content) {
                            Ok(saved_giveaways) => for saved_giveaway in saved_giveaways {
                                if giveaways.get(&saved_giveaway.giveaway.campaign.key).is_none() {
                                    giveaways.insert(saved_giveaway.get_url(), saved_giveaway);
                                }
                            },
                            Err(e) => eprintln!("Can't deserialize save file: {}", e)
                        }
                        Err(e) => eprintln!("Can't read save file: {}", e)
                    }
                },
                Err(e) => eprintln!("Can't open save file: {}", e)
            }

            let mut giveaways = giveaways.drain().map(|(_i, g)| g).collect::<Vec<SearchResult>>();
            if config.update > 0 {
                giveaways.sort_by_key(|g| g.last_updated);
            
                let mut len = 0;
                let mut indexes_to_update: Vec<usize> = giveaways.iter().enumerate().filter(|(_idx, g)| g.last_updated < g.giveaway.campaign.ends_at).map(|(idx, _g)| idx).filter(|_idx| if len < config.update {len += 1; true} else {false}).collect();
                indexes_to_update.reverse();
                
                let mut progress_bar = ProgressBar::new(len);
                for idx in indexes_to_update {
                    progress_bar.set_action("Updating", Color::Blue, Style::Normal);
                    match crate::crawler_lib::gleam::fetch(&giveaways[idx].get_url()) {
                        Ok(updated) => giveaways[idx] = updated,
                        Err(crate::crawler_lib::Error::InvalidResponse) => {
                            progress_bar.print_info("Invalid", &format!("giveaway {} (giveaway has been removed)", giveaways[idx].get_url()), Color::Red, Style::Bold);
                            giveaways.remove(idx);
                        }
                        Err(crate::crawler_lib::Error::Timeout) => {
                            progress_bar.print_info("Timeout", "Failed to load giveaway (giveaway has not been updated)", Color::Red, Style::Bold);
                            sleep(Duration::from_secs(10));
                        }
                    }
                    progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal);
                    progress_bar.inc();
                    sleep(Duration::from_secs(cooldown));
                }
                progress_bar.print_info("Finished", &format!("{} giveaways updated", len), Color::Green, Style::Bold);
                progress_bar.set_action("Finished", Color::Green, Style::Bold);
                progress_bar.finalize();
                println!();
            }

            match File::create("giveaways.json") {
                Ok(mut file) => {
                    match serde_json::to_string(&giveaways) {
                        Ok(data) => match file.write(data.as_bytes()) {
                            Ok(_) => (),
                            Err(e) => eprintln!("Can't write to file: {}", e)
                        },
                        Err(e) => eprintln!("Can't serialize data: {}", e)
                    }
                }
                Err(e) => eprintln!("Can't open save file: {}", e)
            }

            if !fast {
                let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                giveaways.retain(|g| g.giveaway.campaign.ends_at > timestamp);
                
                use meilisearch_sdk::client::Client;
                #[allow(clippy::ptr_arg)]
                async fn update_database(meili_host: &str, meili_key: &str, meili_index: &str, running_giveaways: &Vec<SearchResult>) {
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
                
                update_database(&config.meilisearch.host, &config.meilisearch.key, &config.meilisearch.index, &giveaways).await
            }
        }

        if !fast {
            let time_elapsed = Instant::now().duration_since(start);
            let time_to_sleep = Duration::from_secs(3540) - time_elapsed;
            sleep(time_to_sleep);
        } else {
            break;
        }
    }
}