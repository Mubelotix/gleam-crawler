use gleam_finder::*;
use std::thread;
use std::time::Duration;
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
                .help("Set set waiting time in seconds between two request to the same website.")
        )
        .get_matches();

    let minimal: bool = if matches.occurrences_of("minimal") > 0 {
        true
    } else {
        false
    };

    let cooldown: u64 = matches.value_of("cooldown").unwrap_or("6").parse().unwrap_or(6);

    for page in 0..4 {
        if !minimal { println!("\x1B[0;34mloading google page {}", page); }
        for link in google::search(page) {
            if !minimal { println!("\x1B[0;34mloading {}", link); }
            for gleam_link in intermediary::resolve(&link) {
                if !minimal {
                    println!("\x1B[1;32mgleam link found: {}", gleam_link);
                } else {
                    println!("{}", gleam_link);
                }
            }
            if !minimal { println!("\x1B[0;33msleeping"); }
            thread::sleep(Duration::from_secs(cooldown));
        }
    }
}