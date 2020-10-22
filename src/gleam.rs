use std::time::{SystemTime, UNIX_EPOCH};
use format::parsing::*;
use format::prelude::*;

/// Extract the id of the giveaway from an url.
pub fn get_gleam_id(url: &str) -> Option<&str> {
    if url.len() == 37 && &url[0..30] == "https://gleam.io/competitions/" {
        return Some(&url[30..35]);
    } else if url.len() >= 23 && &url[0..17] == "https://gleam.io/" && &url[22..23] == "/" {
        return Some(&url[17..22]);
    }
    None
}

#[derive(Debug)]
pub enum Error {
    InvalidGleamUrl,
    NetworkError(minreq::Error),
    ParseError(ParseError),
}

pub fn fetch(url: &str) -> Result<SearchResult, Error> {
    let giveaway_id = match get_gleam_id(url) {
        Some(id) => id,
        None => return Err(Error::InvalidGleamUrl),
    };

    let url = format!("https://gleam.io/{}/-", giveaway_id);
    let response = match minreq::get(&url)
        .with_header("Host", "gleam.io")
        .with_header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:72.0) Gecko/20100101 Firefox/72.0",
        )
        .with_header("Accept", "text/html")
        .with_header("DNT", "1")
        .with_header("Connection", "keep-alive")
        .with_header("Upgrade-Insecure-Requests", "1")
        .with_header("TE", "Trailers")
        .send()
    {
        Ok(response) => response,
        Err(e) => {
            return Err(Error::NetworkError(e));
        },
    };

    let body = match response.as_str() {
        Ok(body) => body,
        Err(e) => return Err(Error::NetworkError(e)),
    };

    let (giveaway, entry_count) = match format::parsing::parse_html(body) {
        Ok((giveaway, _, entry_count)) => (giveaway, entry_count),
        Err(e) => return Err(Error::ParseError(e)),
    };
    
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let entry_evolution = match entry_count {
        Some(e) => {
            let mut hashmap = std::collections::HashMap::new();
            hashmap.insert(now, e);
            Some(hashmap)
        },
        None => None
    };
    
    Ok(SearchResult {
        giveaway: giveaway.into(),
        last_updated: now,
        referers: vec![url],
        entry_count,
        entry_evolution,
    })
}
#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};
    use super::*;

    #[test]
    fn test_giveaway_struct() {
        let giveaway =
            fetch("https://gleam.io/29CPn/-2-alok-gveaway-and-12000-diamonds-")
                .unwrap();
        println!("{:?}", giveaway);
            sleep(Duration::from_secs(15));
        let giveaway =
            fetch("https://gleam.io/SB3C7/-")
               .unwrap();
        println!("{:?}", giveaway);
        sleep(Duration::from_secs(15));
        let giveaway = fetch("https://gleam.io/8nTqy/amd-5700xt-gpu").unwrap();
        println!("{:?}", giveaway);
        sleep(Duration::from_secs(15));
        let giveaway =
            fetch("https://gleam.io/ff3QT/win-an-ipad-pro-with-canstar").unwrap();
        println!("{:?}", giveaway);
    }

    #[test]
    fn get_gleam_urls() {
        assert_eq!(
            get_gleam_id("https://gleam.io/competitions/lSq1Q-s"),
            Some("lSq1Q")
        );
        assert_eq!(
            get_gleam_id("https://gleam.io/2zAsX/bitforex-speci"),
            Some("2zAsX")
        );
        assert_eq!(get_gleam_id("https://gleam.io/7qHd6/sorteo"), Some("7qHd6"));
        assert_eq!(
            get_gleam_id("https://gleam.io/3uSs9/taylor-moon"),
            Some("3uSs9")
        );
        assert_eq!(
            get_gleam_id("https://gleam.io/OWMw8/sorteo-de-1850"),
            Some("OWMw8")
        );
        assert_eq!(
            get_gleam_id("https://gleam.io/competitions/CEoiZ-h"),
            Some("CEoiZ")
        );
        assert_eq!(get_gleam_id("https://gleam.io/7qHd6/-"), Some("7qHd6"));
 
    }
}