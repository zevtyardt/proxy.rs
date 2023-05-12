use std::net::IpAddr;

use crate::resolver::{GeoData, Resolver};

#[derive(Debug, Clone)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub proto: Vec<String>,
    pub geo: GeoData,

    pub types: Vec<(String, Option<String>)>,

    timeout: i32,
    runtimes: Vec<f64>,
}

impl Proxy {
    pub async fn new(mut host: String, port: u16, proto: Vec<String>) -> Self {
        let resolver = Resolver::new();
        if !resolver.host_is_ip(&host) {
            host = resolver.resolve(host).await.unwrap();
        }
        let geo = resolver.get_ip_info(host.parse::<IpAddr>().unwrap()).await;
        Proxy {
            host,
            port,
            proto,
            geo,
            types: vec![],
            timeout: 8,
            runtimes: vec![],
        }
    }

    pub fn avg_resp_time(&self) -> f64 {
        if self.runtimes.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.runtimes.iter().sum();
        sum / self.runtimes.len() as f64
    }
}

// TODO ADD TYPES
impl std::fmt::Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Proxy {} {:.2}s {}:{}>",
            self.geo.iso_code,
            self.avg_resp_time(),
            self.host,
            self.port
        )
    }
}

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.port == other.port
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}
