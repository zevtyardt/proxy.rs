use async_recursion::async_recursion;
use std::time::Duration;

use hyper::{client::HttpConnector, header::LOCATION, Body, Client, Request};
use hyper_tls::HttpsConnector;
use regex::Regex;
use tokio::time::timeout;

use crate::{
    providers::{PROXIES, UNIQUE_PROXIES},
    utils::http::{hyper_client, random_useragent},
};

#[derive(Debug, Clone)]
pub struct BaseProvider {
    pub client: Client<HttpsConnector<HttpConnector>>,
    pub proto: Vec<String>,
    pub domain: String,
    pub timeout: i32,
}

impl BaseProvider {
    pub fn build_get_request(&self, uri: String) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("User-Agent", random_useragent(true))
            .body(Body::empty())
            .unwrap()
    }

    #[async_recursion]
    pub async fn get_html(&self, request: Request<Body>) -> String {
        if let Ok(Ok(response)) = timeout(
            Duration::from_secs(self.timeout as u64),
            self.client.request(request),
        )
        .await
        {
            if let Some(redirect_url) = response.headers().get(LOCATION) {
                let request = self.build_get_request(redirect_url.to_str().unwrap().into());

                return self.get_html(request).await;
            }

            if let Ok(body) = hyper::body::to_bytes(response.into_body()).await {
                let body_str = String::from_utf8_lossy(&body);
                return body_str.to_string();
            }
        }
        String::new()
    }

    pub fn find_proxies(&self, pattern: String, html: &str) -> Vec<(String, u16, Vec<String>)> {
        let re = Regex::new(&pattern).unwrap();
        let mut proxies = vec![];
        for cap in re.captures_iter(html) {
            let ip = cap.get(1).unwrap().as_str();
            let port = cap.get(2).unwrap().as_str();

            if let Ok(port) = port.parse::<u16>() {
                proxies.push((ip.to_string(), port, self.proto.clone()))
            }
        }
        proxies
    }

    pub async fn update_stack(&self, proxies: &Vec<(String, u16, Vec<String>)>) {
        let mut added = 0;
        for (ip, port, proto) in proxies {
            let host_port = format!("{}:{}", ip, port);
            let mut unique_proxy = UNIQUE_PROXIES.lock();
            if !unique_proxy.contains(&host_port)
                && PROXIES
                    .push((ip.to_owned(), *port, proto.to_owned()))
                    .is_ok()
            {
                added += 1;
                unique_proxy.push(host_port)
            }
        }
        log::debug!(
            "{} of {} proxies added from {}",
            added,
            proxies.len(),
            self.domain
        );

        //log::debug!("{} proxies added(received) from {}", added, self.domain)
    }
}

impl Default for BaseProvider {
    fn default() -> Self {
        Self {
            client: hyper_client(),
            domain: String::new(),
            timeout: 5,
            proto: vec![],
        }
    }
}
