use std::{collections::HashMap, io::Read};

use autocompress::Decoder;
use httparse::EMPTY_HEADER;
use rand::Rng;

pub fn random_useragent() -> String {
    let mut rng = rand::thread_rng();
    let name = option_env!("CARGO_PKG_NAME").unwrap_or("proxy-rs");
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");
    let rv = rng.gen_range(1000..9999);

    format!("{}/{}/{}", name, version, rv)
}

#[derive(Debug)]
pub struct Response {
    pub version: Option<u8>,
    pub status_code: Option<u16>,
    pub reason: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: String,
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

            for header in parser.headers {
                response.headers.insert(
                    header.name.to_string(),
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
                response.body = String::from_utf8_lossy(&body).to_string();
            }
        }

        response
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            version: None,
            status_code: None,
            reason: None,
            headers: HashMap::new(),
            body: String::new(),
        }
    }
}
