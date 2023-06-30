use std::collections::BTreeMap;

use httparse::{Request, Status, EMPTY_HEADER};
use url::Url;

#[derive(Debug, Default)]
pub struct RequestParser {
    pub method: Option<String>,
    pub path: Option<String>,
    pub version: Option<u8>,
    pub headers: BTreeMap<String, String>,
}

impl RequestParser {
    pub fn parse(data: &[u8]) -> Self {
        let mut request = RequestParser::default();
        let mut chunk = vec![EMPTY_HEADER; 128];
        let mut parser = Request::new(&mut chunk);

        if let Ok(Status::Complete(_)) = parser.parse(data) {
            if let Some(method) = parser.method {
                request.method = Some(method.to_string());
            }
            if let Some(path) = parser.path {
                request.path = Some(path.to_string());
            }
            request.version = parser.version;
            for header in parser.headers {
                request.headers.insert(
                    header.name.to_lowercase().to_string(),
                    String::from_utf8_lossy(header.value).to_string(),
                );
            }
        }

        request
    }

    pub fn get_host(&self) -> Option<String> {
        if let Some(path) = &self.path {
            if let Ok(url) = Url::parse(path.as_str()) {
                if let Some(host) = url.host_str() {
                    return Some(host.to_string());
                }
            }
        }

        if let Some(host) = self.headers.get(&"host".to_string()) {
            return Some(host.to_string());
        }

        None
    }
}
