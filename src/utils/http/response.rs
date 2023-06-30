use autocompress::Decoder;
use httparse::{Response, Status, EMPTY_HEADER};
use std::{collections::BTreeMap, io::Read};

#[derive(Debug, Default)]
pub struct ResponseParser {
    pub version: Option<u8>,
    pub status_code: Option<u16>,
    pub reason: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub body: String,
    pub raw: String,
}

impl ResponseParser {
    pub fn parse(data: &[u8]) -> Self {
        let mut response = ResponseParser::default();
        let mut chunk = vec![EMPTY_HEADER; 128];
        let mut parser = Response::new(&mut chunk);

        if let Ok(Status::Complete(n)) = parser.parse(data) {
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
