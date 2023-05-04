use std::{collections::HashMap, net::IpAddr};

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use maxminddb::{geoip2::City, Reader};
use std::sync::{Arc, Mutex};

use crate::utils::geolite_database::open_geolite_db;

#[derive(Debug)]
pub struct GeoData {
    pub iso_code: String,
    pub name: String,
    pub region_iso_code: String,
    pub region_name: String,
    pub city_name: String,
}

impl GeoData {
    fn default() -> Self {
        let unknown = String::from("unknown");
        GeoData {
            iso_code: String::from("--"),
            name: unknown.clone(),
            region_iso_code: unknown.clone(),
            region_name: unknown.clone(),
            city_name: unknown,
        }
    }
}

lazy_static! {
    pub static ref DNS_RESOLVER: trust_dns_resolver::Resolver = trust_dns_resolver::Resolver::new(
        trust_dns_resolver::config::ResolverConfig::default(),
        trust_dns_resolver::config::ResolverOpts::default(),
    )
    .unwrap();
    pub static ref GEO_CITY: AsyncOnce<Reader<Vec<u8>>> =
        AsyncOnce::new(async { open_geolite_db().await.unwrap() });
    pub static ref CACHED_HOSTS: Arc<Mutex<HashMap<String, String>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone)]
pub struct Resolver {
    ext_ip_hosts: Vec<String>,
}

impl Resolver {
    pub fn new() -> Self {
        let ext_ip_hosts: Vec<String> = vec![
            "https://wtfismyip.com/text",
            "http://api.ipify.org/",
            "http://ipinfo.io/ip",
            "http://ipv4.icanhazip.com/",
            "http://myexternalip.com/raw",
            "http://ipinfo.io/ip",
            "http://ifconfig.io/ip",
        ]
        .iter()
        .map(|x| x.to_string())
        .collect();

        Resolver { ext_ip_hosts }
    }

    pub fn host_is_ip(&self, ipv4: &str) -> bool {
        let ipaddress: Option<IpAddr> = match ipv4.parse() {
            Ok(ip) => Some(ip),
            Err(_) => None,
        };
        ipaddress.is_some()
    }

    pub async fn get_ip_info(&self, ip_address: IpAddr) -> GeoData {
        let mut geodata = GeoData::default();
        if let Ok(lookup) = GEO_CITY.get().await.lookup::<City>(ip_address) {
            if let Some(country) = &lookup.country {
                if let Some(country_iso_code) = &country.iso_code {
                    geodata.iso_code = country_iso_code.to_string()
                }
                if let Some(country_names) = &country.names {
                    if let Some(country_name) = country_names.get("en") {
                        geodata.name = country_name.to_string();
                    }
                }
            }

            if let Some(continent) = &lookup.continent {
                if let Some(continent_iso_code) = &continent.code {
                    geodata.iso_code = continent_iso_code.to_string()
                }
                if let Some(continent_names) = &continent.names {
                    if let Some(continent_name) = continent_names.get("en") {
                        geodata.name = continent_name.to_string();
                    }
                }
            }

            if let Some(subdivisions) = &lookup.subdivisions {
                if let Some(division) = subdivisions.first() {
                    if let Some(division_iso_code) = &division.iso_code {
                        geodata.region_iso_code = division_iso_code.to_string()
                    }
                    if let Some(division_names) = &division.names {
                        if let Some(division_name) = division_names.get("en") {
                            geodata.region_name = division_name.to_string()
                        }
                    }
                }
            }

            if let Some(city) = &lookup.city {
                if let Some(city_names) = &city.names {
                    if let Some(city_name) = city_names.get("en") {
                        geodata.city_name = city_name.to_string()
                    }
                }
            }
        }
        geodata
    }

    pub fn resolve(&self, host: String) -> Option<String> {
        if self.host_is_ip(&host) {
            return Some(host);
        }

        if let Some(cached_host) = CACHED_HOSTS.lock().unwrap().get(&host) {
            log::debug!("host {} is already cached, returning", host);
            return Some(cached_host.to_string());
        }

        match DNS_RESOLVER.lookup_ip(&host) {
            Ok(response) => {
                if let Some(ip) = response.iter().next() {
                    log::info!("resolving host {}: {}", host, ip);
                    CACHED_HOSTS.lock().unwrap().insert(host, ip.to_string());
                    return Some(ip.to_string());
                } else {
                    log::error!("host ({}) is empty", host);
                }
            }
            Err(e) => log::error!("failed to resolve: {}, {}", host, e),
        }
        None
    }

    pub async fn get_real_ext_ip(&self) -> Option<String> {
        for ext_ip_host in &self.ext_ip_hosts {
            match reqwest::get(ext_ip_host).await {
                Ok(response) => match response.text().await {
                    Ok(body) => {
                        let ip = body.trim();
                        if self.host_is_ip(ip) {
                            log::info!("ext ip ({}) retrieved using host: {}", ip, ext_ip_host);
                            return Some(ip.to_string());
                        }
                    }
                    Err(e) => log::error!("{}", e),
                },
                Err(e) => log::error!("{}", e),
            }
        }
        None
    }
}
