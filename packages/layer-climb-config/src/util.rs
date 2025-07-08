use anyhow::Result;
use url::Url;

pub fn set_port_in_url(input_url: &str, new_port: u16) -> Result<String> {
    // Check if the URL has a scheme; if not, add a default one for parsing
    let has_scheme = input_url.contains("://");
    let url_with_scheme = if has_scheme {
        input_url.to_string()
    } else {
        format!("http://{input_url}")
    };

    // Parse the URL
    let mut url = Url::parse(&url_with_scheme)?;

    // Set or replace the port
    url.set_port(Some(new_port)).unwrap();

    // Remove the scheme if it was originally absent
    let final_url = if has_scheme {
        url.to_string()
    } else {
        url[url::Position::AfterScheme..].to_string()
    };

    Ok(final_url)
}
