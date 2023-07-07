use crate::{proxy::Proxy, resolver::GeoData};
use concurrent_queue::ConcurrentQueue;
use lazy_static::lazy_static;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BinaryHeap, VecDeque},
};

lazy_static! {
    pub static ref LIVE_PROXIES: ConcurrentQueue<Proxy> = ConcurrentQueue::bounded(20);
}

#[derive(Debug, Clone)]
pub struct SimpleProxy {
    pub host: String,
    pub port: u16,
    pub geo: GeoData,
    pub types: Vec<(String, Option<String>)>,
    pub schemes: Vec<String>,

    pub runtimes: Vec<f64>,
    pub request_stat: i32,
    pub error_stat: BTreeMap<String, i32>,
}
impl SimpleProxy {
    pub fn as_text(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn error_rate(&self) -> f64 {
        if self.request_stat == 0 {
            return 0.0;
        }
        let sum = self.error_stat.values().sum::<i32>() as f64;
        sum / self.request_stat as f64
    }

    pub fn avg_resp_time(&self) -> f64 {
        if self.runtimes.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.runtimes.iter().sum();
        sum / self.runtimes.len() as f64
    }

    pub fn get_schemes(&mut self) -> Vec<String> {
        if self.schemes.is_empty() {
            for (proxy_type, _) in &self.types {
                if !self.schemes.contains(&"HTTP".to_string())
                    && ["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"].contains(&proxy_type.as_str())
                {
                    self.schemes.push("HTTP".to_string());
                }
                if !self.schemes.contains(&"HTTPS".to_string())
                    && ["HTTPS", "SOCKS4", "SOCKS5"].contains(&proxy_type.as_str())
                {
                    self.schemes.push("HTTPS".to_string());
                }
            }
        }
        self.schemes.clone()
    }
}

impl Ord for SimpleProxy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.error_rate()
            .partial_cmp(&other.error_rate())
            .unwrap()
            .reverse()
            .then(
                self.avg_resp_time()
                    .partial_cmp(&other.avg_resp_time())
                    .unwrap()
                    .reverse(),
            )
    }
}

impl PartialOrd for SimpleProxy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for SimpleProxy {}

impl PartialEq for SimpleProxy {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.port == other.port
    }
}

#[derive(Debug)]
pub struct ProxyPool {
    pool: BinaryHeap<SimpleProxy>,
    newcomers: VecDeque<SimpleProxy>,

    pub strategy: String,
    pub min_req_proxy: i32,
    pub max_error_rate: f64,
    pub max_avg_resp_time: f64,
    pub min_queue: i32,
}

impl ProxyPool {
    pub fn new() -> Self {
        Self {
            pool: BinaryHeap::new(),
            newcomers: VecDeque::new(),
            strategy: "best".to_string(),
            min_req_proxy: 5,
            max_error_rate: 0.5,
            max_avg_resp_time: 8.0,
            min_queue: 5,
        }
    }

    pub fn get(&mut self, schemes: &str) -> Option<SimpleProxy> {
        let scheme = schemes.to_uppercase();
        if self.pool.len() + self.newcomers.len() < self.min_queue as usize {
            return self.import(&scheme);
        } else if !self.newcomers.is_empty() {
            return self.newcomers.pop_front();
        } else if self.strategy == "best" {
            let mut cache = VecDeque::new();
            while !self.pool.is_empty() {
                if let Some(mut proxy) = self.pool.pop() {
                    if proxy.get_schemes().contains(&scheme) {
                        self.pool.extend(cache);
                        return Some(proxy);
                    } else {
                        cache.push_back(proxy)
                    }
                } else {
                    break;
                }
            }
            self.pool.extend(cache);
            return self.import(&scheme);
        }
        None
    }

    pub fn import(&mut self, expected_schemes: &String) -> Option<SimpleProxy> {
        loop {
            if let Ok(proxy) = LIVE_PROXIES.pop() {
                let mut proxy = SimpleProxy {
                    host: proxy.host.clone(),
                    port: proxy.port,
                    geo: proxy.geo.clone(),
                    types: proxy.types.clone(),
                    schemes: proxy.schemes.clone(),
                    runtimes: proxy.runtimes.clone(),
                    request_stat: proxy.request_stat,
                    error_stat: proxy.error_stat.clone(),
                };
                if !proxy.get_schemes().contains(expected_schemes) {
                    self.put(proxy)
                } else {
                    return Some(proxy);
                }
            }
        }
    }

    pub fn put(&mut self, proxy: SimpleProxy) {
        let is_exceed_time = proxy.error_rate() > self.max_error_rate
            || proxy.avg_resp_time() > self.max_avg_resp_time;

        if proxy.request_stat < self.min_req_proxy {
            log::debug!("{} added to newcomers", proxy.as_text());
            self.newcomers.push_back(proxy)
        } else if proxy.request_stat >= self.min_req_proxy && is_exceed_time {
            log::debug!("{} removed from ProxyPool", proxy.as_text());
        } else {
            log::debug!("{} added to pool", proxy.as_text());
            self.pool.push(proxy)
        }
    }

    pub fn remove(&mut self, host: &str, port: u16) -> Option<SimpleProxy> {
        for index in 0..self.newcomers.len() {
            let proxy = self.newcomers.pop_front().unwrap();
            if proxy.host == host && proxy.port == port {
                self.newcomers.remove(index);
                return Some(proxy);
            } else {
                self.newcomers.push_back(proxy)
            }
        }
        let mut cache = VecDeque::new();
        while !self.pool.is_empty() {
            if let Some(proxy) = self.pool.pop() {
                if proxy.host == host && proxy.port == port {
                    self.pool.extend(cache);
                    return Some(proxy);
                } else {
                    cache.push_back(proxy)
                }
            } else {
                break;
            }
        }
        self.pool.extend(cache);

        None
    }
}
