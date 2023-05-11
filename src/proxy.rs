use std::net::IpAddr;

use crate::resolver::{GeoData, Resolver};

#[derive(Debug, Clone)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub proto: Vec<String>,
    pub geo: GeoData,
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
            geo,
            proto,
        }
    }
}

impl std::fmt::Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Proxy {}:{}>", self.host, self.port)
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
