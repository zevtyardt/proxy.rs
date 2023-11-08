pub struct Proxy {
    host: String,
    port: u16,
    types: Vec<String>,
    is_working: bool,
}

impl Proxy {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            types: vec!["HTTP: High".into()],
            is_working: false,
        }
    }
}

impl std::fmt::Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Proxy ID [{}] {}:{}>",
            self.types.join(", "),
            self.host,
            self.port
        )
    }
}
