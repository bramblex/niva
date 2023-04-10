use url::Url;

#[cfg(target_os = "windows")]
pub fn make_base_url(protocol: &str, host: &str) -> String {
    format!("https://{}.{}", protocol, host)
}

#[cfg(target_os = "macos")]
pub fn make_base_url(protocol: &str, host: &str) -> String {
    format!("{}://{}", protocol, host)
}

pub fn get_host_from_url(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    let scheme = url.scheme();
    let host = url.host_str()?;
    Some(format!("{}://{}", scheme, host))
}
