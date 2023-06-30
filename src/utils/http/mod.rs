use rand::Rng;
use std::collections::BTreeMap;
pub mod request;
pub mod response;

pub fn random_useragent(random_value: bool) -> String {
    let name = option_env!("CARGO_PKG_NAME").unwrap_or("proxy-rs");
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");

    let mut rv = "".to_string();
    if random_value {
        let mut rng = rand::thread_rng();
        rv.push('/');
        rv.push_str(rng.gen_range(1000..9999).to_string().as_str())
    }

    format!("{}/{}{}", name, version, rv)
}

pub fn get_headers(random_value: bool) -> (BTreeMap<String, String>, String) {
    let ua = random_useragent(random_value);

    let ua_c = ua.clone();
    let rv = ua_c.split('/').last().unwrap();
    let mut headers = BTreeMap::new();

    headers.insert("User-Agent".to_string(), ua);
    headers.insert("Accept".to_string(), "*/*".to_string());
    headers.insert("Accept-Encoding".to_string(), "gzip, deflate".to_string());
    headers.insert("Pragma".to_string(), "no-cache".to_string());
    headers.insert("Cache-Control".to_string(), "no-cache".to_string());
    headers.insert("Cookie".to_string(), "cookie=ok".to_string());
    headers.insert("Referer".to_string(), "https://google.com/".to_string());

    (headers, rv.to_string())
}
