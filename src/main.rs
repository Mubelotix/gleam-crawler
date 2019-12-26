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
use serde::{Deserialize, Serialize};
use gleam_finder::gleam::Giveaway;
use csv::Reader;

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
                .help("Update the file giveaways.csv with new values and delete invalid old giveaways. Enable --advanced option.")
        )
        .arg(
            Arg::with_name("advanced")
                .long("advanced")
                .short("a")
                .help("Scan gleam.io to get informations like number of entries, name and description of the giveaway.")
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
    let advanced: bool = if matches.occurrences_of("advanced") > 0 || save {
        true
    } else {
        false
    };

    let cooldown: u64 = matches.value_of("cooldown").unwrap_or("6").parse().unwrap_or(6);
    env::set_var("MINREQ_TIMEOUT", matches.value_of("timeout").unwrap_or("6"));

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
                        progress_bar.print_info("Found", &format!("{} ({} entries) => {}", giveaway.get_name(), giveaway.get_entry_count(), giveaway.get_url()), Color::LightGreen, Style::Bold);
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
            }
            timeout_check.insert(results[link_idx].get_host(), Instant::now());
        }
    }
    
    if save {
        let mut records: Vec<Record> = Vec::new();

        if let Ok(mut reader) = Reader::from_path("giveaways.csv") {
            for record in reader.deserialize() {
                if let Ok(record) = record {
                    records.push(record);
                }
            }
        }
    }
}