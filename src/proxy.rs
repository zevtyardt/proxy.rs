use crate::resolver::Resolver;

#[derive(Debug, Clone)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
}

impl Proxy {
    pub async fn new(mut host: String, port: u16) -> Self {
        let resolver = Resolver::new();
        if !resolver.host_is_ip(&host) {
            host = resolver.resolve(host).await.unwrap();
        }

        Proxy { host, port }
    }
}

impl std::fmt::Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Proxy {}:{}>", self.host, self.port)
    }
}
