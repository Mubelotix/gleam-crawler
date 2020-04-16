# gleam_finder

This is a simple CLI program gathering gleam.io's links in a JSON file. The program supports many options and is designed to help for making search engines using [MeiliSearch](https://www.meilisearch.com/). 
This program is using [my crate](https://crates.io/crates/gleam_finder), which provides useful structs and methods related to gleam.io crawling and is used by [Googleam](https://googleam.mubelotix.dev), the gleam.io's search engine.

## Advantages

* lightweight
* efficient
* stable
* cross-platform

## How to build

Like the majority of Rust programs, simply use `cargo build` or `cargo run` (and add `--release` to enable optimization). Note that you can download the binaries [here](https://github.com/Mubelotix/gleam_finder_client/releases).

## How to use

Run `./gleam_finder_client --help` to print the help page. Then run the program again with the flags which you need. If you want to use [MeiliSearch](https://www.meilisearch.com/), you have to run the MeiliSearch server independently.

## Updating

Note that updating can erase your entire database contained in the file `giveaways.json`.

You have to pay attention to the version number:
* If only the last number is increased (ex: 0.2.3 to 0.2.5), you can upgrade directly,
* If the first or the second number is increased (ex: 0.2.5 to 0.4.1), it may break your database. Instructions for upgrading to the next major version are given [here](https://github.com/Mubelotix/gleam_finder_client/releases).