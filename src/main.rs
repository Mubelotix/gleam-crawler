use std::env;
use clap::*;
use serde::{Deserialize, Serialize};
mod error;
mod stats;
mod crawler;
mod config;
mod google;
mod gleam;
use config::*;
use stats::*;
use crawler::launch;

fn backup() {

}

fn init_meilisearch() {

}

fn configurate() {

}

#[tokio::main]
async fn main() {
    let matches = clap_app!(myapp =>
        (version: "4.0")
        (author: "Mubelotix <mubelotix@gmail.com>")
        (about: "Crawl the web to find gleam.io links")
        (@arg CONFIG: -c --config +takes_value "Sets a custom config file")
        (@subcommand stats =>
            (about: "Display stats about the database")
        )
        (@subcommand init_meilisearch =>
            (about: "Init the meilisearch index")
        )
        (@subcommand configurate =>
            (about: "Build a configuration file")
        )
        (@subcommand backup =>
            (about: "Backup the database")
        )
        (@subcommand launch =>
            (about: "Launch the bot")
            (@arg fast: -f --f "Do not load gleam.io pages and do not save them")
        )
    ).get_matches();

    let config = read_config(matches.value_of("CONFIG").unwrap_or("config.toml"));

    match matches.subcommand() {
        ("stats", Some(_args)) => stats(config),
        ("init_meilisearch", Some(_args)) => init_meilisearch(),
        ("configurate", Some(_args)) => configurate(),
        ("backup", Some(_args)) => backup(),
        ("launch", Some(args)) => {
            let fast: bool = args.value_of("fast").unwrap_or("false").parse().unwrap();
            launch(config, fast).await;
        },
        (name, Some(_args)) => {
            println!("Unknown subcommand: {:?}", name);
        }
        (_name, None) => {
            println!("No subcommand, no action taken");
        }
    }

    return;
    
    /*let client = Client::new(meili_host, meili_key);
    let index = match client.get_or_create(meili_index).await {
        Ok(index) => index,
        Err(e) => {
            eprintln!("Meilisearch error while initializing the index: {:?}", e);
            return;
        },
    };
    if let Err(e) = index.set_searchable_attributes(&["name", "description"]).await {
        eprintln!("Meilisearch error while setting searchable attributes: {:?}", e);
        return;
    };
    if let Err(e) = index.set_stop_words(&["the", "to", "of", "a", "in", "it", "on", "at", "an"]).await {
        eprintln!("Meilisearch error while setting stop words: {:?}", e);
        return;
    };*/
}