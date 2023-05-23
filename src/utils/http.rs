use std::{collections::HashMap, io::Read};

use autocompress::Decoder;
use httparse::EMPTY_HEADER;
use rand::Rng;

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

pub fn get_headers(random_value: bool) -> (HashMap<String, String>, String) {
    let ua = random_useragent(random_value);

    let ua_c = ua.clone();
    let rv = ua_c.split('/').last().unwrap();
    let mut headers = HashMap::new();

    headers.insert("User-Agent".to_string(), ua);
    headers.insert("Accept".to_string(), "*/*".to_string());
    headers.insert("Accept-Encoding".to_string(), "gzip, deflate".to_string());
    headers.insert("Pragma".to_string(), "no-cache".to_string());
    headers.insert("Cache-Control".to_string(), "no-cache".to_string());
    headers.insert("Cookie".to_string(), "cookie=ok".to_string());
    headers.insert("Referer".to_string(), "https://google.com/".to_string());

    (headers, rv.to_string())
}

#[derive(Debug, Default)]
pub struct Response {
    pub version: Option<u8>,
    pub status_code: Option<u16>,
    pub reason: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub raw: String,
}

impl Response {
    pub fn parse(data: &[u8]) -> Self {
        let mut response = Response::default();
        let mut chunk = vec![EMPTY_HEADER; 128];
        let mut parser = httparse::Response::new(&mut chunk);

        if let Ok(httparse::Status::Complete(n)) = parser.parse(data) {
            response.version = parser.version;
            response.status_code = parser.code;
            response.reason = Some(parser.reason.unwrap().to_string());
            response.raw.push_str(&String::from_utf8_lossy(&data[..n]));

            for header in parser.headers {
                response.headers.insert(
                    header.name.to_string().to_lowercase(),
                    String::from_utf8_lossy(header.value).to_string(),
                );
            }
            let body = &data[n..];
            if !match Decoder::suggest(body) {
                Ok(mut decoder) => match decoder.read_to_string(&mut response.body) {
                    Err(e) => {
                        log::debug!("Error: {}", e);
                        false
                    }
                    _ => true,
                },
                Err(e) => {
                    log::debug!("Error: {}", e);
                    false
                }
            } {
                response.body = String::from_utf8_lossy(body).to_string();
            }
            response.raw.push_str("\r\n\r\n");
            response.raw.push_str(response.body.as_str());
        } else {
            response.raw.push_str(&String::from_utf8_lossy(data))
        }
        response
    }
}
