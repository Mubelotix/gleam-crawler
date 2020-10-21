use string_tools::{get_all_after, get_all_between_strict};

#[derive(Debug)]
pub enum Error {
    NetworkError(minreq::Error),
    Utf8Error,
}

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
                eprintln!("Failed to load google search page: {}", e);
                return Err(Error::NetworkError(e));
            }
    };

    let mut body = match response.as_str() {
        Ok(body) => body,
        Err(e) => {
            eprintln!("Failed to read google search page: {}", e);
            return Err(Error::Utf8Error);
        }
    };
    
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
}

fn get_full_url(page: usize) -> String {
    format!(
        "https://www.google.com/search?q=\"gleam.io\"&tbs=qdr:h&filter=0&start={}",
        page * 10
    )
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