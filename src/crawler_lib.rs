use format::prelude::*;

#[derive(Debug)]
pub enum Error {
    Timeout,
    InvalidResponse,
}

/// Contains functions related to google pages parsing.
pub mod google {
    use super::Error;
    use string_tools::{get_all_after, get_all_between_strict};

    fn get_full_url(page: usize) -> String {
        format!(
            "https://www.google.com/search?q=\"gleam.io\"&tbs=qdr:h&filter=0&start={}",
            page * 10
        )
    }

    /// Search google for a something and returns result urls.  
    /// See [Google Advanced Search](https://www.google.com/advanced_search) for more information about request syntax.  
    /// Only one page is loaded.  
    /// # Examples
    /// ```
    /// use gleam_finder::google;
    ///
    /// // note that we only test the first page of google results and that there can be more
    /// let links = google::search(0);
    /// ```
    pub fn search(page: usize) -> Result<Vec<String>, Error> {
        let response = match minreq::get(get_full_url(page))
            .with_header("Accept", "text/plain")
            .with_header("Host", "www.google.com")
            .with_header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:71.0) Gecko/20100101 Firefox/71.0",
            )
            .send() {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return Err(Error::Timeout);
                }
        };
        
        if let Ok(mut body) = response.as_str() {
            /*use std::io::prelude::*;  // useful for debugging
            use std::fs::File;
            let mut file = File::create(format!("page{}.html", page)).unwrap();
            file.write_all(body.as_bytes()).unwrap();*/
            let mut rep = Vec::new();
            while let Some(url) =
                get_all_between_strict(body, "\"><a href=\"", "\"")
            {
                body = get_all_after(body, url);
                if body.starts_with("\" onmousedown=\"return rwt(") || body.starts_with("\" data-ved=\"2a") {
                    rep.push(url.to_string());
                }
            }
            Ok(rep)
        } else {
            Err(Error::InvalidResponse)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn get_full_url_test() {
            assert_eq!(
                "https://www.google.com/search?q=\"gleam.io\"&tbs=qdr:h&filter=0&start=10",
                get_full_url(1)
            );
        }

        #[test]
        fn resolve_google_request() {
            let result = search(0).unwrap();
            assert!(!result.is_empty());

            let result = search(9).unwrap();
            assert!(result.is_empty());
        }
    }
}

pub mod intermediary {
    use super::Error;
    use super::gleam::get_gleam_id;
    use string_tools::{get_all_after, get_all_between};

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

    pub fn resolve(url: &str) -> Result<Vec<String>, Error> {
        match minreq::get(url)
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
            Ok(response) => {
                if let Ok(mut body) = response.as_str() {
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
                        if let Some(id) = get_gleam_id(&url) {
                            final_rep.push(format!("https://gleam.io/{}/-", id));
                        }
                    }
                    Ok(final_rep)
                } else {
                    Err(Error::InvalidResponse)
                }
            },
            Err(_e) => {
                Err(Error::Timeout)
            },
        }
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
}

/// Contains giveaways fetcher
pub mod gleam {
    use format::prelude::*;
    use super::Error;
    use serde_json::{from_str, Value};
    use std::thread::sleep;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use string_tools::{get_all_between_strict, get_idx_between_strict};

    /// Extract the id of the giveaway from an url.
    pub fn get_gleam_id(url: &str) -> Option<&str> {
        if url.len() == 37 && &url[0..30] == "https://gleam.io/competitions/" {
            return Some(&url[30..35]);
        } else if url.len() >= 23 && &url[0..17] == "https://gleam.io/" && &url[22..23] == "/" {
            return Some(&url[17..22]);
        }
        None
    }

    pub fn fetch(url: &str) -> Result<SearchResult, Error> {
        let giveaway_id = match get_gleam_id(url) {
            Some(id) => id,
            None => return Err(Error::InvalidResponse),
        };
        let url = format!("https://gleam.io/{}/-", giveaway_id);

        if let Ok(response) = minreq::get(&url)
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
            if let Ok(body) = response.as_str() {
                if let Some(json) = get_all_between_strict(
                    body,
                    "<div class='popup-blocks-container' ng-init='initCampaign(",
                    ")'>",
                ) {
                    let json = json.replace("&quot;", "\"");
                    let entry_count: Option<usize> = if let Some(entry_count) =
                    get_all_between_strict(body, "initEntryCount(", ")")
                    {
                        if let Ok(entry_count) = entry_count.parse() {
                            Some(entry_count)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let giveaway: format::giveaway::Giveaway = match serde_json::from_str(&json) {
                        Ok(giveaway) => giveaway,
                        Err(e) => {
                            eprintln!("Error while parsing giveaway: {}", e);
                            return Err(Error::InvalidResponse);
                        },
                    };

                    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    let entry_evolution = match entry_count {
                        Some(e) => {
                            let mut hashmap = std::collections::HashMap::new();
                            hashmap.insert(now, e);
                            Some(hashmap)
                        },
                        None => None
                    };
                    
                    return Ok(SearchResult {
                        giveaway: giveaway.into(),
                        last_updated: now,
                        referers: vec![url],
                        entry_count,
                        entry_evolution,
                    });
                }
            }
            Err(Error::InvalidResponse)
        } else {
            Err(Error::Timeout)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_giveaway_struct() {
            let giveaway =
                fetch("https://gleam.io/29CPn/-2-alok-gveaway-and-12000-diamonds-")
                    .unwrap();
            println!("{:?}", giveaway);

            sleep(Duration::from_secs(5));

            let giveaway = fetch("https://gleam.io/8nTqy/amd-5700xt-gpu").unwrap();
            println!("{:?}", giveaway);

            sleep(Duration::from_secs(5));

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
}
