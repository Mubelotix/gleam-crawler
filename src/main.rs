use gleam_finder::*;
use std::thread;
use std::time::Duration;
use std::env;
use progress_bar::progress_bar::ProgressBar;
use progress_bar::color::{Color, Style};
use clap::*;

fn main() {
    let matches = App::new("Gleam finder")
        .version("1.1")
        .author("Mubelotix <mubelotix@gmail.com>")
        .about("Search for gleam links on the web.")
        .arg(
            Arg::with_name("minimal")
                .long("minimal")
                .short("m")
                .help("Enables simplified mode")
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

    let cooldown: u64 = matches.value_of("cooldown").unwrap_or("6").parse().unwrap_or(6);
    env::set_var("MINREQ_TIMEOUT", matches.value_of("timeout").unwrap_or("6"));

    if !minimal {
        let mut progress_bar = ProgressBar::new(10);
        progress_bar.set_action("Searching", Color::White, Style::Normal);

        let mut results: Vec<String> = Vec::new();
        let mut page = 0;
        loop {
            progress_bar.print_info("Getting", &format!("the results page {}", page), Color::Blue, Style::Normal);
            let mut new_results = google::search(page);
            if new_results.len() > 0 {
                results.append(&mut new_results);
                page += 1;
                progress_bar.inc();
                progress_bar.print_info("Sleeping", &format!("for {} seconds", cooldown), Color::Yellow, Style::Normal);
                thread::sleep(Duration::from_secs(cooldown));
            } else {
                break;
            }
        }

        let mut progress_bar = ProgressBar::new(results.len());
        progress_bar.set_action("Loading", Color::White, Style::Normal);
        for link in results {
            progress_bar.print_info("Loading", &link, Color::Blue, Style::Normal);
            for gleam_link in intermediary::resolve(&link) {
                progress_bar.print_info("Found", &gleam_link, Color::LightGreen, Style::Bold);
            }
            progress_bar.inc();
            progress_bar.print_info("Sleeping", &format!("for {} seconds", cooldown), Color::Yellow, Style::Normal);
            thread::sleep(Duration::from_secs(cooldown));
        }
    }
    
}