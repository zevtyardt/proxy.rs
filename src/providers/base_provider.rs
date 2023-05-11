use regex::Regex;
use reqwest::{Client, RequestBuilder};

use crate::{
    proxy::Proxy,
    utils::{http::random_useragent, queue::FifoQueue, run_parallel, CustomFuture},
};

#[derive(Debug)]
pub struct BaseProvider<'a> {
    pub proxies: FifoQueue<Proxy>,
    pub proto: Vec<&'a str>,
    pub client: Client,
}

impl BaseProvider<'_> {
    pub async fn get_html(&self, task: RequestBuilder) -> String {
        if let Ok(response) = task.send().await {
            if let Ok(body) = response.text().await {
                return body;
            }
        }
        String::new()
    }

    pub async fn get_all_html(&self, tasks: Vec<RequestBuilder>) -> Vec<String> {
        let mut mapped_tasks: Vec<CustomFuture<String>> = vec![];
        for task in tasks {
            let fut = Box::pin(async {
                if let Ok(response) = task.send().await {
                    if let Ok(body) = response.text().await {
                        return body;
                    }
                }
                String::new()
            });
            mapped_tasks.push(fut);
        }

        run_parallel::<String>(mapped_tasks, 5) // TODO: configurable concurrent
            .await
    }

    pub fn find_proxies(&self, pattern: &str, html: &str) -> Vec<(String, u16, Vec<&str>)> {
        let re = Regex::new(pattern).unwrap();
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
        proxies
    }
}

impl Default for BaseProvider<'_> {
    fn default() -> Self {
        BaseProvider {
            proxies: FifoQueue::new(),
            client: Client::builder()
                .user_agent(random_useragent())
                .build()
                .unwrap(),
            proto: vec![],
        }
    }
}
