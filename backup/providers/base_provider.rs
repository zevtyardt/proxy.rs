use async_recursion::async_recursion;
use std::time::Duration;

use hyper::{client::HttpConnector, header::LOCATION, Body, Client, Request};
use hyper_tls::HttpsConnector;
use regex::Regex;
use tokio::time::timeout;

use crate::utils::{
    http::{hyper_client, random_useragent},
    vec_of_strings,
};

#[derive(Debug, Clone)]
pub struct Provider {
    pub url: &'static str,
    pub new_urls: Option<fn(&String, String) -> Vec<String>>,
    pub max_depth: u32,
    pub pattern: &'static str,
    pub proto: Vec<String>,
    pub name: &'static str,
    pub timeout: i32,
}

impl Default for Provider {
    fn default() -> Self {
        Self {
            pattern: r#"(?P<ip>(?:\d+\.?){4})\:(?P<port>\d+)"#,
            url: "",
            name: "",
            new_urls: None,
            max_depth: 1,
            timeout: 5,
            proto: vec_of_strings![
                "HTTP",
                "HTTPS",
                "SOCKS4",
                "SOCKS5",
                "CONNECT:80",
                "CONNECT:25"
            ],
        }
    }
}
pub struct ProviderTask {
    client: Client<HttpsConnector<HttpConnector>>,
    base: Provider,
}

impl ProviderTask {
    pub fn new(base: Provider) -> Self {
        Self {
            client: hyper_client(),
            base,
        }
    }
    fn build_get_request(&self, uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("User-Agent", random_useragent(true))
            .body(Body::empty())
            .unwrap()
    }

    #[async_recursion]
    async fn get_html(&self, request: Request<Body>) -> String {
        if let Ok(Ok(response)) = timeout(
            Duration::from_secs(self.base.timeout as u64),
            self.client.request(request),
        )
        .await
        {
            let (part, body) = response.into_parts();
            let location = part.headers.get(LOCATION);
            if let Some(redirect_url) = location {
                let redirect_url = redirect_url.to_str().unwrap();
                let request = self.build_get_request(redirect_url);
                return self.get_html(request).await;
            }

            if let Ok(body) = hyper::body::to_bytes(body).await {
                let body_str = String::from_utf8_lossy(&body);
                return body_str.to_string();
            }
        }
        String::new()
    }

    pub async fn get_proxies(&self) -> Vec<(String, u16, Vec<String>)> {
        let mut all_proxies = vec![];
        let mut urls = vec![self.base.url.to_string()];
        let mut url_cache = urls.clone();
        let mut depth = 0;
        let re = Regex::new(self.base.pattern).unwrap();

        while let Some(url) = urls.pop() {
            let request = self.build_get_request(&url);
            let html = self.get_html(request).await;

            if depth < self.base.max_depth {
                if let Some(find_urls) = self.base.new_urls {
                    let host = if let Ok(parsed_url) = url::Url::parse(self.base.url) {
                        parsed_url.scheme().to_owned() + "://" + parsed_url.host_str().unwrap()
                    } else {
                        "http://".to_owned() + self.base.name
                    };
                    for url in find_urls(&html, host) {
                        if !url_cache.contains(&url) {
                            urls.push(url.clone());
                            url_cache.push(url);
                        }
                    }
                }
                depth += 1;
            }

            for cap in re.captures_iter(&html) {
                let ip = cap.get(1).unwrap().as_str();
                let port = cap.get(2).unwrap().as_str();

                if let Ok(port) = port.parse::<u16>() {
                    all_proxies.push((ip.to_string(), port, self.base.proto.clone()));
                }
            }
        }
        all_proxies
    }
}
