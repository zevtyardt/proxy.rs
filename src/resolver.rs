use std::{collections::HashMap, net::IpAddr};

use include_dir::{include_dir, Dir};
use maxminddb::{geoip2::City, Reader};

static DATA_DIR: Dir = include_dir!("src/data/");

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

// TODO: implement debug
pub struct Resolver<'a> {
    geo_city: Reader<&'a [u8]>,
    dns_resolver: trust_dns_resolver::Resolver,
    cached_hosts: HashMap<String, String>,
    ext_ip_hosts: Vec<&'a str>,
}

impl Resolver<'_> {
    pub fn new() -> Self {
        let geolite2_mmdb = DATA_DIR.get_file("GeoLite2-City.mmdb").unwrap();
        let geo_city = Reader::from_source(geolite2_mmdb.contents()).unwrap();

        let dns_resolver = trust_dns_resolver::Resolver::new(
            trust_dns_resolver::config::ResolverConfig::default(),
            trust_dns_resolver::config::ResolverOpts::default(),
        )
        .unwrap();

        let ext_ip_hosts = vec![
            "https://wtfismyip.com/text",
            "http://api.ipify.org/",
            "http://ipinfo.io/ip",
            "http://ipv4.icanhazip.com/",
            "http://myexternalip.com/raw",
            "http://ipinfo.io/ip",
            "http://ifconfig.io/ip",
        ];

        Resolver {
            geo_city,
            dns_resolver,
            cached_hosts: HashMap::new(),
            ext_ip_hosts,
        }
    }

    pub fn host_is_ip(&self, ipv4: &str) -> bool {
        let ipaddress: Option<IpAddr> = match ipv4.parse() {
            Ok(ip) => Some(ip),
            Err(_) => None,
        };
        ipaddress.is_some()
    }

    pub fn get_ip_info(&self, ip_address: IpAddr) -> GeoData {
        let mut geodata = GeoData::default();
        if let Ok(lookup) = self.geo_city.lookup::<City>(ip_address) {
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

    pub fn resolve(&mut self, host: &str) -> Option<String> {
        if self.host_is_ip(host) {
            return Some(host.to_string());
        }

        if let Some(cached_host) = self.cached_hosts.get(host) {
            return Some(cached_host.to_string());
        }

        if let Ok(response) = self.dns_resolver.lookup_ip(host) {
            if let Some(ip) = response.iter().next() {
                self.cached_hosts.insert(host.to_string(), ip.to_string());
                return Some(ip.to_string());
            }
        }
        None
    }

    pub fn get_real_ext_ip(&self) -> Option<String> {
        for ext_ip_host in &self.ext_ip_hosts {
            if let Ok(response) = reqwest::blocking::get(ext_ip_host.to_string()) {
                if let Ok(body) = response.text() {
                    let ip = body.trim();
                    if self.host_is_ip(ip) {
                        return Some(ip.to_string());
                    }
                }
            }
        }
        None
    }
}
