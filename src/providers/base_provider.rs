use std::time::Duration;

use futures_util::future::join_all;
use regex::Regex;
use reqwest::{Client, RequestBuilder};

use crate::{providers::PROXIES, proxy::Proxy, utils::http::random_useragent};

#[derive(Debug, Clone)]
pub struct BaseProvider {
    pub proto: Vec<String>,
    pub domain: String,
    pub client: Client,
    pub max_retry: i32,
}

impl BaseProvider {
    pub fn start(&mut self) {
        log::debug!("Try to get proxies from {}", self.domain);
    }

    pub async fn get_html(&self, task: RequestBuilder) -> String {
        for _ in 0..self.max_retry {
            let task_c = task.try_clone().unwrap();
            if let Ok(response) = task_c.send().await {
                if let Ok(body) = response.text().await {
                    return body;
                }
            }
        }
        String::new()
    }

    pub async fn get_all_html(&self, tasks: Vec<RequestBuilder>) -> Vec<String> {
        let mut mapped_tasks = vec![];
        for task in tasks {
            let fut = tokio::task::spawn(async {
                if let Ok(response) = task.send().await {
                    if let Ok(body) = response.text().await {
                        return body;
                    }
                }
                String::new()
            });
            mapped_tasks.push(fut);
        }

        join_all(mapped_tasks)
            .await
            .into_iter()
            .map(|f| f.unwrap())
            .collect()
    }

    pub fn find_proxies(&self, pattern: String, html: &str) -> Vec<(String, u16, Vec<String>)> {
        let re = Regex::new(&pattern).unwrap();
        let mut proxies = vec![];
        for cap in re.captures_iter(html) {
            let ip = cap.get(1).unwrap().as_str();
            let port = cap.get(2).unwrap().as_str();
            proxies.push((
                ip.to_string(),
                port.parse::<u16>().unwrap(),
                self.proto.clone(),
            ))
        }
        log::debug!("{} proxies received from {}", proxies.len(), self.domain);
        proxies
    }

    pub async fn update_stack(&self, proxies: &Vec<(String, u16, Vec<String>)>) {
        let mut added = 0;
        for (ip, port, proto) in proxies {
            let proxy = Proxy::create(ip, *port, proto.to_vec()).await;
            let is_added = PROXIES.push_unique(proxy);

            if is_added {
                added += 1;
            }
        }

        log::debug!("{} proxies added(received) from {}", added, self.domain)
    }
}

impl Default for BaseProvider {
    fn default() -> Self {
        BaseProvider {
            client: Client::builder()
                .user_agent(random_useragent())
                .timeout(Duration::from_secs(5)) // todo customable
                .build()
                .unwrap(),
            domain: String::new(),
            max_retry: 3,
            proto: vec![],
        }
    }
}
