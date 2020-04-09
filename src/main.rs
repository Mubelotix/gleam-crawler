use url::Host;
use gleam_finder::*;
use std::thread;
use std::time::Duration;
use std::env;
use progress_bar::progress_bar::ProgressBar;
use progress_bar::color::{Color, Style};
use clap::*;
use url::Url;
use std::collections::HashMap;
use std::time::Instant;
use gleam_finder::gleam::Giveaway;
use serde::{Deserialize, Serialize};
use serde_json::{to_string, from_str};
use std::fs::File;
use std::io::prelude::*;

fn fix_str_size(mut input: String, size: usize) -> String {
    return if input.chars().count() < size {
        while input.chars().count() < size {
            input.push(' ');
        }
        input
    } else if input.chars().count() > size {
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
    } else {
        input
    };
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
        return Host::Domain("undefined");
    }

    fn get_url(&self) -> &str {
        &self.url
    }
}

fn main() {
    let matches = App::new("Gleam finder")
        .version("1.1")
        .author("Mubelotix <mubelotix@gmail.com>")
        .about("Search for gleam links on the web.")
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
                .help("Update the file giveaways.json with new values and delete invalid old giveaways. Enable --advanced option.")
        )
        .arg(
            Arg::with_name("advanced")
                .long("advanced")
                .short("a")
                .help("Scan gleam.io to get informations like number of entries, name and description of the giveaway.")
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
                .help("Set the waiting time in seconds between two request to the same website.")
        )
        .arg(
            Arg::with_name("timeout")
                .long("timeout")
                .takes_value(true)
                .min_values(0)
                .max_values(3600)
                .default_value("6")
                .help("Set the timeout for a request.")
        )
        .get_matches();

    let minimal: bool = if matches.occurrences_of("minimal") > 0 {
        true
    } else {
        false
    };
    let force_cooldown: bool = if matches.occurrences_of("force-cooldown") > 0 {
        true
    } else {
        false
    };
    let save: bool = if matches.occurrences_of("save") > 0 {
        true
    } else {
        false
    };
    let auto_enter: bool = if matches.occurrences_of("auto-enter") > 0 {
        true
    } else {
        false
    };
    let advanced: bool = if matches.occurrences_of("advanced") > 0 || save || auto_enter {
        true
    } else {
        false
    };
    let loop_enabled: bool = if matches.occurrences_of("loop") > 0 {
        true
    } else {
        false
    };

    let cooldown: u64 = matches.value_of("cooldown").unwrap_or("6").parse().unwrap_or(6);
    env::set_var("MINREQ_TIMEOUT", matches.value_of("timeout").unwrap_or("6"));
    

    loop {
        let mut giveaways = HashMap::new();
        let start = Instant::now();

        if !minimal {
            let mut progress_bar = ProgressBar::new(7);
            progress_bar.set_action("Searching", Color::White, Style::Normal);
    
            let mut results = Vec::new();
            let mut page = 0;
            loop {
                progress_bar.set_action("Loading", Color::Blue, Style::Normal);
                progress_bar.print_info("Getting", &format!("the results page {}", page), Color::Blue, Style::Normal);
                let new_results = google::search(page);
                if new_results.len() > 0 {
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
                for gleam_link in intermediary::resolve(results[link_idx].get_url()) {
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
                        if let Some(giveaway) = Giveaway::fetch(&gleam_link) {
                            last_gleam_request = Instant::now();
                            progress_bar.print_info("Found", &format!("{} {} => {}", fix_str_size(giveaway.get_name().to_string(), 40), fix_str_size(format!("({:?} entries)", giveaway.get_entry_count()), 18), giveaway.get_url()), Color::LightGreen, Style::Bold);
                            giveaways.insert(gleam_link, giveaway);
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
                let new_results = google::search(page);
                if new_results.len() > 0 {
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
                
                for gleam_link in intermediary::resolve(results[link_idx].get_url()) {
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
    
                        if let Some(giveaway) = Giveaway::fetch(&gleam_link) {
                            last_gleam_request = Instant::now();
                            giveaways.insert(gleam_link, giveaway);
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
                                if let None = giveaways.get(saved_giveaway.get_gleam_id()) {
                                    giveaways.insert(saved_giveaway.get_url().to_string(), saved_giveaway);
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