use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Geo {
    pub country: Country,
    pub region: Region,
    pub city: String,
}

#[derive(Debug, Serialize)]
pub struct Country {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Region {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ProxyData {
    pub host: String,
    pub port: u16,
    pub geo: Geo,
    pub types: Vec<ProxyType>,
    pub avg_resp_time: f64,
    pub error_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct ProxyType {
    pub proxy_type: String,
    pub level: Option<String>,
}
