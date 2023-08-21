use std::{collections::BTreeMap, process::exit, sync::Arc, time::Duration};

use dashmap::{DashMap, DashSet};
use futures_util::{stream::FuturesUnordered, StreamExt};
use lazy_static::lazy_static;
use rand::{seq::SliceRandom, thread_rng};
use regex::Regex;
use tokio::{sync::Semaphore, time};

use crate::{
    judge::{check_judge_host, get_judges, Judge},
    negotiators::{
        connect_25::Connect25Negotiator, connect_80::Connect80Negotiator, http::HttpNegotiator,
        https::HttpsNegotiator, socks4::Socks4Negotiator, socks5::Socks5Negotiator,
    },
    proxy::Proxy,
    resolver::Resolver,
    utils::{
        geolite_database::DOWNLOADING,
        http::{get_headers, response::ResponseParser},
        vec_of_strings,
    },
};

lazy_static! {
    static ref ENABLE_PROTOCOLS: DashSet<String> = {
        let mut set = DashSet::new();
        set.extend(vec_of_strings!["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"]);
        set.insert(String::from("HTTPS"));
        set.insert(String::from("CONNECT:25"));
        set
    };
    static ref JUDGES: DashMap<String, Vec<Judge>> = DashMap::new();
}

pub async fn check_judges(ssl: bool, ext_ip: String, mut expected_types: Vec<String>) {
    let stime = time::Instant::now();
    if !expected_types.contains(&"SMTP".to_string())
        && expected_types.contains(&"CONNECT:25".to_string())
    {
        expected_types.push("SMTP".to_string());
    }

    if !expected_types.contains(&"HTTP".to_string())
        && ["CONNECT:80", "SOCKS4", "SOCKS5"]
            .iter()
            .any(|x| expected_types.contains(&x.to_string()))
    {
        expected_types.push("HTTP".to_string());
    }

    let mut futures = FuturesUnordered::new();
    let sem = Arc::new(Semaphore::new(20));

    for mut judge in get_judges() {
        let permit = Arc::clone(&sem).acquire_owned().await;
        let expected_types = expected_types.clone();
        let ext_ip = ext_ip.clone();
        let ssl = ssl;
        futures.push(tokio::spawn(async move {
            let _ = permit;
            if expected_types.contains(&judge.scheme) {
                judge.verify_ssl = ssl;
                check_judge_host(&mut judge, &ext_ip).await;
            }
            judge
        }));
    }

    let mut working = 0;
    let no_judges = DashSet::new();
    let disable_protocols = DashSet::new();

    while let Some(Ok(judge)) = futures.next().await {
        if judge.is_working {
            if !JUDGES.contains_key(&judge.scheme) {
                JUDGES.insert(judge.scheme.clone(), vec![]);
            }
            if let Some(mut v) = JUDGES.get_mut(&judge.scheme) {
                v.push(judge);
                working += 1;
            }
        } else {
            if expected_types.contains(&judge.scheme) {
                no_judges.insert(judge.scheme.clone());
            }

            if judge.scheme == "HTTP" {
                for protocol in vec_of_strings!["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"] {
                    ENABLE_PROTOCOLS.remove(&protocol);
                    disable_protocols.insert(protocol);
                }
            } else if judge.scheme == "SMTP" {
                let protocol = String::from("CONNECT:25");
                ENABLE_PROTOCOLS.remove(&protocol);
                disable_protocols.insert(protocol);
            } else {
                let protocol = String::from("HTTPS");
                ENABLE_PROTOCOLS.remove(&protocol);
                disable_protocols.insert(protocol);
            }
        }
    }

    if !no_judges.is_empty() {
        log::warn!(
            "Not found judges for the {:?} schemes. Checking proxy on protocols {:?} is disabled.",
            no_judges
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<String>>(),
            disable_protocols
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<String>>(),
        );
    }
    if working == 0 || expected_types.into_iter().all(|f| no_judges.contains(&f)) {
        log::error!("No judges found!");
        while *DOWNLOADING.lock() {
            continue;
        }
        exit(0);
    }
    log::info!("{} judges added, Runtime {:?}", working, stime.elapsed());
}

#[derive(Clone, Debug)]
pub struct Checker {
    pub verify_ssl: bool,
    pub timeout: i32,
    pub max_tries: i32,
    pub method: String,

    pub support_referer: bool,
    pub support_cookie: bool,

    pub expected_types: Vec<String>,
    pub expected_levels: Vec<String>,
    pub expected_countries: Vec<String>,

    pub ext_ip: String,
    ip_re: Regex,
}

impl Checker {
    pub async fn check_proxy(&mut self, proxy: &mut Proxy) -> bool {
        let expected_types = vec_of_strings![
            "CONNECT:80",
            "CONNECT:25",
            "SOCKS5",
            "SOCKS4",
            "HTTPS",
            "HTTP"
        ]; // proxy.expected_types.clone();

        let mut result = vec![];
        for proto in &expected_types {
            if self.expected_types.contains(proto)
                && ENABLE_PROTOCOLS.contains(proto)
                && (self.expected_countries.is_empty()
                    || self.expected_countries.contains(&proxy.geo.iso_code))
            {
                let mut is_working = false;
                for _ in 0..self.max_tries {
                    is_working = self.check_proto(proxy, proto).await;
                    if is_working {
                        break;
                    }
                }
                if proto == "HTTP" && is_working && !self.expected_levels.is_empty() {
                    is_working = proxy.types.iter().any(|(_, level)| {
                        level.is_some() && self.expected_levels.contains(&level.clone().unwrap())
                    });
                }
                result.push(is_working)
            }
        }

        proxy.is_working = result.iter().any(|i| *i);
        proxy.is_working
    }

    pub async fn check_proto(&mut self, proxy: &mut Proxy, proto: &String) -> bool {
        proxy.negotiator_proto = proto.to_string();
        let mut is_working = false;
        if let Some(judge) = self.get_judge(proto) {
            proxy.log(format!("Selected judge: {}", judge).as_str(), None, None);

            if proto != "HTTPS" && !proxy.connect().await {
                proxy.close().await;
                return false;
            }

            let (negotiate_success, use_full_path, check_anon_lvl) =
                self.negotiate(proxy, &judge, proto).await;
            if !negotiate_success {
                proxy.close().await;
                return false;
            }

            if proto == "CONNECT:25" {
                proxy.types.push((proto.to_string(), None));
                return true;
            }

            let path = judge.url.path().to_string();
            let (raw_request, headers, rv) =
                self.build_raw_request(&judge.host, &path, use_full_path, None);

            proxy.send(raw_request.as_bytes()).await;
            if let Some(data) = proxy.recv_all().await {
                proxy.log("Request: success", None, None);
                let mut anonimity_lvl = None;
                let response = ResponseParser::parse(data.as_slice());

                //log::warn!("=====\n{raw_request}\n{0}", response.raw);

                if self.get_response_status(&response, headers, rv) {
                    if check_anon_lvl {
                        anonimity_lvl = Some(self.get_anonimity_level(&response, &judge.marks));
                    }

                    is_working = true;
                    proxy.types.push((proto.to_string(), anonimity_lvl));
                }
                proxy.close().await;
            } else {
                proxy.log("Request: failed", None, Some("request_failed".to_string()));
            }
        }

        is_working
    }

    async fn negotiate(
        &self,
        proxy: &mut Proxy,
        judge: &Judge,
        proto: &String,
    ) -> (bool, bool, bool) {
        if proto == "CONNECT:25" {
            let negotiator = Connect25Negotiator::default();
            (
                negotiator.negotiate(proxy, judge).await,
                negotiator.use_full_path,
                negotiator.check_anon_lvl,
            )
        } else if proto == "CONNECT:80" {
            let negotiator = Connect80Negotiator::default();
            (
                negotiator.negotiate(proxy, judge).await,
                negotiator.use_full_path,
                negotiator.check_anon_lvl,
            )
        } else if proto == "SOCKS5" {
            let negotiator = Socks5Negotiator::default();
            (
                negotiator.negotiate(proxy).await,
                negotiator.use_full_path,
                negotiator.check_anon_lvl,
            )
        } else if proto == "SOCKS4" {
            let negotiator = Socks4Negotiator::default();
            (
                negotiator.negotiate(proxy).await,
                negotiator.use_full_path,
                negotiator.check_anon_lvl,
            )
        } else if proto == "HTTPS" {
            let negotiator = HttpsNegotiator::default();
            (
                negotiator.negotiate(proxy, judge).await,
                negotiator.use_full_path,
                negotiator.check_anon_lvl,
            )
        } else if proto == "HTTP" {
            let negotiator = HttpNegotiator::default();
            (
                negotiator.negotiate().await,
                negotiator.use_full_path,
                negotiator.check_anon_lvl,
            )
        } else {
            (false, false, false)
        }
    }

    fn get_anonimity_level(
        &self,
        response: &ResponseParser,
        marks: &BTreeMap<String, usize>,
    ) -> String {
        let content = response.body.to_lowercase();
        let mut via = false;
        if let Some(via_m) = marks.get("via") {
            via = content.matches("via").count() > *via_m
        }
        if let Some(proxy_m) = marks.get("proxy") {
            if !via {
                via = content.replace("proxy-rs", "--").matches("proxy").count() > *proxy_m
            }
        }

        let all_ips: Vec<String> = self
            .ip_re
            .find_iter(&content)
            .map(|f| f.as_str().to_string())
            .collect();

        if all_ips.contains(&self.ext_ip) {
            "Transparent".to_string()
        } else if via {
            "Anonymous".to_string()
        } else {
            "High".to_string()
        }
    }

    fn get_response_status(
        &self,
        response: &ResponseParser,
        headers: BTreeMap<String, String>,
        rv: String,
    ) -> bool {
        let response_raw = response.raw.to_lowercase();
        let version_is_correct = response_raw.contains(&rv);
        let support_referer = if self.support_referer {
            if let Some(referer) = headers.get("Referer") {
                response_raw.contains(&referer.to_lowercase())
            } else {
                false
            }
        } else {
            true
        };
        let support_cookie = if self.support_cookie {
            if let Some(cookie) = headers.get("Cookie") {
                response_raw.contains(&cookie.to_lowercase())
            } else {
                false
            }
        } else {
            true
        };
        let some_ip = self.ip_re.find(&response_raw).is_some();
        let is_ok = response.status_code.unwrap_or(0) == 200;

        is_ok && version_is_correct && some_ip && support_referer && support_cookie
    }

    fn build_raw_request(
        &self,
        host: &String,
        path: &String,
        use_full_path: bool,
        data: Option<String>,
    ) -> (String, BTreeMap<String, String>, String) {
        let mut request = if use_full_path {
            format!("{} http://{}{} HTTP/1.1\r\n", self.method, host, path)
        } else {
            format!("{} {} HTTP/1.1\r\n", self.method, path)
        };

        let (mut headers, rv) = get_headers(true);
        let data = data.unwrap_or("".to_string());
        headers.insert("Host".to_string(), host.to_string());
        headers.insert("Connection".to_string(), "close".to_string());
        headers.insert("Content-Length".to_string(), data.len().to_string());
        if self.method == "POST" {
            headers.insert(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            );
        }
        for (k, v) in headers.iter() {
            request += format!("{}: {}\r\n", k, v).as_str();
        }
        request += "\r\n";
        request += data.as_str();

        (request, headers, rv)
    }

    fn get_judge(&mut self, proto: &str) -> Option<Judge> {
        let mut scheme = "HTTP".to_string();
        if proto.eq("HTTPS") {
            scheme = "HTTPS".to_string();
        } else if proto.eq("CONNECT:25") {
            scheme = "SMTP".to_string();
        }

        let t = time::Instant::now();
        while !JUDGES.contains_key(&scheme) {
            if t.elapsed() >= Duration::from_secs(15) {
                log::error!("Timeout error: no judges found");
                while *DOWNLOADING.lock() {
                    continue;
                }
                exit(0)
            }
        }

        if let Some(v) = JUDGES.get_mut(&scheme) {
            if let Some(judge) = v.choose(&mut thread_rng()) {
                return Some(judge.clone());
            }
        }

        None
    }
}

impl Checker {
    pub async fn new() -> Self {
        let resolver = Resolver::new();
        Checker {
            verify_ssl: false,
            timeout: 5,
            max_tries: 3,
            method: "GET".to_string(),
            support_cookie: false,
            support_referer: false,
            expected_types: vec![],
            expected_countries: vec![],
            expected_levels: vec![],
            ip_re: Regex::new(r#"\d+\.\d+\.\d+\.\d+"#).unwrap(),
            ext_ip: resolver.get_real_ext_ip().await,
        }
    }
}
