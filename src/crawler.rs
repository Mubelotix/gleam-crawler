use crate::{config::*, google, gleam, meilisearch::*, database::*};
use std::{collections::HashMap, time::{Instant, Duration, SystemTime}, thread::sleep};
use progress_bar::{color::*, progress_bar::ProgressBar};
use url::Url;
use format::{prelude::*, parsing::*};

fn url_to_host(url: &str) -> String {
    if let Ok(url) = &Url::parse(url) {
        url.host_str().unwrap_or("unknown").to_string()
    } else {
        "unknown".to_string()
    }
}

fn search_google_results(cooldown: u64) -> Vec<String> {
    let mut progress_bar = ProgressBar::new(7);
    progress_bar.set_action("Searching", Color::White, Style::Normal);
    let mut results = Vec::new();
    let mut page = 0;
    loop {
        progress_bar.set_action("Loading", Color::Blue, Style::Normal);
        progress_bar.print_info("Getting", &format!("the results page {}", page), Color::Blue, Style::Normal);
        let new_results = google::search(page).unwrap_or_default();
        if !new_results.is_empty() {
            for new_result in new_results {
                results.push(new_result);
            }
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
    println!();

    results
}

fn load_results(results: Vec<String>, config: &Config, giveaways: &mut HashMap<String, SearchResult>, outdated_meilisearch: &mut Vec<String>, fast: bool) {
    let cooldown = config.cooldown as u64;

    let mut progress_bar = ProgressBar::new(results.len());
    let mut timeout_check = HashMap::new();
    let mut last_gleam_request = Instant::now();
    progress_bar.set_action("Loading", Color::White, Style::Normal);
    for result in &results {
        // Check the cooldown
        if let Some(last_load_time) = timeout_check.get(&url_to_host(&result)) {
            let time_since_last_load = Instant::now() - *last_load_time;
            if time_since_last_load < Duration::from_secs(cooldown) {
                let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal); 
                sleep(time_to_sleep);
            }
        }
        
        // Load the page
        progress_bar.set_action("Loading", Color::Blue, Style::Normal);
        let giveaway_urls = match resolve(result) {
            Ok(urls) => urls,
            Err(e) => {
                progress_bar.print_info("Error", &format!("when trying to load {}: {}", result, e), Color::Red, Style::Normal);
                continue;
            }
        };

        // Blame the page if asked
        if giveaway_urls.is_empty() && config.blame_useless_pages {
            progress_bar.print_info("Useless", &format!("page loaded: {}", result), Color::Yellow, Style::Normal);
        }

        // Use the data
        for gleam_link in giveaway_urls {
            // Check if the url is valid and if we did not load this before
            if let Some(key) = gleam::get_gleam_id(&gleam_link) {
                if giveaways.contains_key(key) {
                    continue;
                }
            } else {
                continue;
            }

            if !fast {
                let time_since_last_load = Instant::now() - last_gleam_request;
                if time_since_last_load < Duration::from_secs(cooldown) {
                    let time_to_sleep = Duration::from_secs(cooldown) - time_since_last_load;
                    progress_bar.set_action("Sleeping", Color::Yellow, Style::Normal);
                    sleep(time_to_sleep);
                }

                progress_bar.set_action("Loading", Color::Blue, Style::Normal);
                if let Ok(giveaway) = gleam::fetch(&gleam_link) {
                    last_gleam_request = Instant::now();
                    progress_bar.print_info("Found", &format!("{} {:>8} entries - {}", giveaway.get_url(), if let Some(entry_count) = giveaway.entry_count { entry_count.to_string() } else {String::from("unknow")}, giveaway.get_name()), Color::LightGreen, Style::Bold);
                    outdated_meilisearch.push(giveaway.giveaway.campaign.key.clone());
                    giveaways.insert(giveaway.giveaway.campaign.key.clone(), giveaway);
                    
                }
            } else {
                progress_bar.print_info("Found", &gleam_link, Color::LightGreen, Style::Bold);
            }
        }
        
        progress_bar.inc();
        timeout_check.insert(url_to_host(result), Instant::now());
    }
    progress_bar.set_action("Finished", Color::Green, Style::Bold);
    progress_bar.print_info("Finished", &format!("{} giveaways found", giveaways.len()), Color::Green, Style::Bold);
    progress_bar.finalize();
    println!();
}

fn update_giveaways(to_update: Vec<String>, giveaways: &mut HashMap<String, SearchResult>, outdated_meilisearch: &mut Vec<String>, cooldown: u64) {
    let len = to_update.len();
    let mut progress_bar = ProgressBar::new(len);
    for key in to_update {
        progress_bar.set_action("Updating", Color::Blue, Style::Normal);
        let mut old_giveaway = giveaways.remove(&key).unwrap();
        outdated_meilisearch.push(key.clone());

        match gleam::fetch(&old_giveaway.get_url()) {
            Ok(updated) => {
                let giveaway = old_giveaway + updated;
                giveaways.insert(key, giveaway);
            },
            Err(gleam::Error::ParseError(ParseError::GiveawayJsonNotFound)) => {
                progress_bar.print_info("Missing", &format!("giveaway {} -> removing", old_giveaway.get_url()), Color::Red, Style::Blink);
            }
            Err(gleam::Error::ParseError(e)) => {
                progress_bar.print_info("Invalid", &format!("giveaway {}: {:?}", old_giveaway.get_url(), e), Color::Red, Style::Blink);
                old_giveaway.last_updated = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                giveaways.insert(key, old_giveaway);
            }
            Err(gleam::Error::NetworkError(_e)) => {
                progress_bar.print_info("Timeout", "Failed to load giveaway (giveaway has not been updated)", Color::Yellow, Style::Bold);
                giveaways.insert(key, old_giveaway);
                sleep(Duration::from_secs(10));
            }
            Err(gleam::Error::InvalidGleamUrl) => {
                progress_bar.print_info("Invalid", &format!("url {} (this code is almost unreachable)", old_giveaway.get_url()), Color::Red, Style::Blink);
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

pub async fn launch(config: Config, fast: bool) {
    std::env::set_var("MINREQ_TIMEOUT", config.timeout.to_string());
    let cooldown = config.cooldown as u64;

    if matches!(config.meilisearch.as_ref().map(|m| m.init_on_launch), Some(true)) {
        println!("Initializing the MeiliSearch index...");
        init_meilisearch(&config).await;
        println!("Done!");
    }
    
    loop {
        let mut giveaways: HashMap<String, SearchResult> = HashMap::new();
        let mut outdated_meilisearch = Vec::new();
        let start = Instant::now();

        // Search results on google
        let results = search_google_results(cooldown);

        // Load the results
        load_results(results, &config, &mut giveaways, &mut outdated_meilisearch, fast);

        if fast { break; }

        // Read the database
        read_database(&mut giveaways, &config);

        // Select the oldest giveaways to update them
        let mut to_update = Vec::new();
        if config.update > 0 {
            let mut giveaways = giveaways.iter().map(|(_i, g)| g).collect::<Vec<&SearchResult>>();
            giveaways.sort_by_key(|g| g.last_updated);
            for idx in 0..config.update {
                if let Some(giveaway) = giveaways.get(idx) {
                    to_update.push(giveaway.giveaway.campaign.key.clone())
                }
            }
        }
        
        // Update the oldest giveaways
        update_giveaways(to_update, &mut giveaways, &mut outdated_meilisearch, cooldown);

        // Save the database
        save_database(&giveaways, &config);

        // Update meilisearch index
        update_meilisearch(giveaways, &config, outdated_meilisearch).await;

        if !fast {
            let time_elapsed = Instant::now().duration_since(start);
            let time_to_sleep = Duration::from_secs(3540) - time_elapsed;
            sleep(time_to_sleep);
        } else {
            break;
        }
    }
}

/// put an url+noise, get url (without http://domain.something/)
fn get_url(url: &str) -> &str {
    let mut i = 0;
    for c in url.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '/' && c != '_' {
            break;
        }
        i += 1;
    }
    &url[..i]
}

pub fn resolve(url: &str) -> Result<Vec<String>, minreq::Error> {
    use string_tools::*;

    let response = match minreq::get(url)
        .with_header("Accept", "text/html,text/plain")
        .with_header(
            "User-Agent",
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:78.0) Gecko/20100101 Firefox/78.0",
        )
        .with_header(
            "Host",
            get_all_between(url, "://", "/"),
        )
        .send()
    {
        Ok(response) => response,
        Err(e) => return Err(e),
    };

    let mut body = match response.as_str() {
        Ok(body) => body,
        Err(e) => return Err(e),
    };

    let mut rep = Vec::new();
    while get_all_after(&body, "https://gleam.io/") != "" {
        let url = get_url(get_all_after(&body, "https://gleam.io/"));
        body = get_all_after(&body, "https://gleam.io/");
        let url = if url.len() >= 20 {
            format!("https://gleam.io/{}", &url[..20])
        } else if !url.is_empty() {
            format!("https://gleam.io/{}", url)
        } else {
            continue;
        };
        if !rep.contains(&url) {
            rep.push(url);
        }
    }
    let mut final_rep = Vec::new();
    for url in rep {
        if let Some(id) = crate::gleam::get_gleam_id(&url) {
            final_rep.push(format!("https://gleam.io/{}/-", id));
        }
    }
    Ok(final_rep)
}

#[cfg(test)]
mod test {
    use super::resolve;

    #[test]
    fn resolving() {
        assert_eq!(resolve("https://www.youtube.com/watch?v=-DS1qgHjoJY").unwrap().len(), 1);
        assert_eq!(resolve("https://news.nestia.com/detail/Oculus-Quest-2---Infinite-Free-Games!/5222508").unwrap().len(), 1);
    }
}