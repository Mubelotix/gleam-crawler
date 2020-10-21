use string_tools::{get_all_after, get_all_between_strict};

pub fn search(page: usize) -> Result<Vec<String>, minreq::Error> {
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
                return Err(e);
            }
    };

    let mut body = match response.as_str() {
        Ok(body) => body,
        Err(e) => {
            eprintln!("Failed to read google search page: {}", e);
            return Err(e);
        }
    };
    
    let mut rep = Vec::new();
    loop
    {
        let mut option1 = get_all_between_strict(body, "\"><a href=\"", "\"")
            .map(|url| (Some(url), get_all_after(body, url)));
        if let Some((Some(_url1), body1)) = option1 {
            if !body1.starts_with("\" onmousedown=\"return rwt(") {
                option1 = Some((None, body1));
            }
        }
        let mut option2 = get_all_between_strict(body, "\" href=\"", "\"")
            .map(|url| (Some(url), get_all_after(body, url)));
        if let Some((_url2, body2)) = option2 {
            if !body2.starts_with("\" data-ved=\"2a") {
                option2 = Some((None, body2));
            }
        }

        let (url, new_body) = match (option1, option2) {
            (Some(option1), None) => option1,
            (None, Some(option2)) => option2,
            (None, None) => break,
            (Some((Some(url1), body1)), Some((None, _body2))) => (Some(url1), body1),
            (Some((None, _body1)), Some((Some(url2), body2))) => (Some(url2), body2),
            (Some((url1, body1)), Some((_url2, body2))) if body1.len() > body2.len() => (url1, body1),
            (Some((_url1, _body1)), Some((url2, body2))) => (url2, body2),
        };

        body = new_body;
        if let Some(url) = url {
            let url = url.to_string();
            if !rep.contains(&url) {
                rep.push(url);
            }
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