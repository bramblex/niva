use url::Url;

pub fn get_host_from_url(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    let scheme = url.scheme();
    let host = url.host_str()?;
    Some(format!("{}://{}", scheme, host))
}
