use std::{collections::BTreeMap, process::exit, sync::Arc};

use futures_util::{stream, StreamExt};
use lazy_static::lazy_static;
use parking_lot::{Condvar, Mutex};
use rand::seq::SliceRandom;
use regex::Regex;
use tokio::{spawn, time};

use crate::{
    judge::{get_judges, Judge},
    negotiators::{
        connect_25::Connect25Negotiator, connect_80::Connect80Negotiator, http::HttpNegotiator,
        https::HttpsNegotiator, socks4::Socks4Negotiator, socks5::Socks5Negotiator,
    },
    proxy::Proxy,
    resolver::Resolver,
    utils::{
        http::{get_headers, response::ResponseParser},
        vec_of_strings,
    },
};

lazy_static! {
    static ref DISABLE_PROTOCOLS: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    static ref JUDGES: Arc<Mutex<BTreeMap<String, Vec<Judge>>>> =
        Arc::new(Mutex::new(BTreeMap::new()));
    static ref CV: Arc<Condvar> = Arc::new(Condvar::new());
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

    stream::iter(get_judges())
        .map(|mut judge| {
            judge.verify_ssl = ssl;

            let ext_ip = ext_ip.clone();
            let expected_types = expected_types.clone();

            spawn(async move {
                let scheme = judge.scheme.clone();
                if !expected_types.contains(&scheme) {
                    return;
                }

                judge.check_host(ext_ip.as_str()).await;
                if judge.is_working {
                    let mut judges_by_scheme = JUDGES.lock();
                    if !judges_by_scheme.contains_key(&scheme) {
                        judges_by_scheme.insert(scheme.clone(), vec![]);
                    }
                    if let Some(value) = judges_by_scheme.get_mut(&scheme) {
                        value.push(judge.clone());
                    }
                }
            })
        })
        .map(|f| async { f.await.unwrap_or(()) })
        .buffer_unordered(5)
        .collect::<Vec<()>>()
        .await;

    let mut working = 0;
    let mut no_judges = vec![];
    for (scheme, proto) in [
        (
            "HTTP",
            vec_of_strings!["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"],
        ),
        ("HTTPS", vec_of_strings!["HTTPS"]),
        ("SMTP", vec_of_strings!["CONNECT:25"]),
    ] {
        if !expected_types.contains(&scheme.to_string()) {
            continue;
        }
        let judges_by_scheme = JUDGES.lock();
        if let Some(value) = judges_by_scheme.get(&scheme.to_string()) {
            if !value.is_empty() {
                working += value.len();
                continue;
            }
        }
        no_judges.push(scheme);
        let mut disable_protocols = DISABLE_PROTOCOLS.lock();
        disable_protocols.extend(proto);
        CV.notify_one();
    }

    if !no_judges.is_empty() {
        log::warn!(
            "Not found judges for the {:?} protocol. Checking proxy on protocols {:?} is disabled.",
            no_judges,
            DISABLE_PROTOCOLS.lock()
        );
    }
    if working == 0 {
        log::error!("Not found judges!");
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
                && !DISABLE_PROTOCOLS.lock().contains(proto)
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
        let judge = self.get_judge(proto);
        proxy.log(format!("Selected judge: {}", judge).as_str(), None, None);

        let mut is_working = false;
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

    fn get_judge(&mut self, proto: &str) -> Judge {
        let mut scheme = "HTTP".to_string();
        if proto.eq("HTTPS") {
            scheme = "HTTPS".to_string();
        } else if proto.eq("CONNECT:25") {
            scheme = "SMTP".to_string();
        }

        let mut judges = JUDGES.lock();
        while !judges.contains_key(&scheme) {
            CV.wait(&mut judges)
        }

        let judges = judges.get(&scheme).unwrap();
        let mut rng = rand::thread_rng();
        judges.choose(&mut rng).unwrap().clone()
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
