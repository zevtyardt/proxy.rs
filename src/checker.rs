use std::{collections::BTreeMap, process::exit, sync::Arc};

use parking_lot::{Condvar, Mutex};
use rand::seq::SliceRandom;
use regex::Regex;
use tokio::{spawn, time};

use crate::{
    judge::{get_judges, Judge},
    negotiators::{
        http_negotiator::HttpNegotiator, https_negotiator::HttpsNegotiator,
        socks4_negotiator::Socks4Negotiator,
    },
    proxy::Proxy,
    resolver::Resolver,
    utils::{
        http::{get_headers, Response},
        run_parallel, vec_of_strings,
    },
};

#[derive(Clone, Debug)]
pub struct Checker {
    pub verify_ssl: bool,
    pub timeout: i32,
    pub max_tries: i32,
    pub method: String,

    disable_protocols: Arc<Mutex<Vec<String>>>,
    judges: Arc<Mutex<BTreeMap<String, Vec<Judge>>>>,
    ext_ip: String,
    ip_re: Regex,
    cv: Arc<Condvar>,
}

impl Checker {
    pub async fn check_judges(&mut self) {
        let stime = time::Instant::now();

        let ext_ip = self.ext_ip.clone();
        let mut tasks = vec![];
        for mut judge in get_judges() {
            let ssl = self.verify_ssl;
            let ext_ip = ext_ip.clone();
            tasks.push(spawn(async move {
                judge.set_verify_ssl(ssl);
                judge.check_host(ext_ip.as_str()).await;
                judge
            }))
        }

        let all_judges = run_parallel::<Judge>(tasks, None).await;

        let mut working = 0;
        for judge in all_judges {
            if judge.is_none() {
                continue;
            }

            let judge = judge.unwrap();
            if judge.is_working {
                let mut judges_by_scheme = self.judges.lock();
                let scheme = &judge.scheme;
                if !judges_by_scheme.contains_key(scheme) {
                    judges_by_scheme.insert(scheme.clone(), vec![]);
                }
                if let Some(value) = judges_by_scheme.get_mut(scheme) {
                    value.push(judge.clone());
                    working += 1;
                }
            }
        }
        let mut no_judges = vec![];
        for (scheme, proto) in [
            (
                "HTTP",
                vec_of_strings!["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"],
            ),
            ("HTTPS", vec_of_strings!["HTTPS"]),
            ("SMTP", vec_of_strings!["SMTP"]),
        ] {
            let judges_by_scheme = self.judges.lock();
            if let Some(value) = judges_by_scheme.get(&scheme.to_string()) {
                if !value.is_empty() {
                    continue;
                }
            }
            no_judges.push(scheme);
            let mut disable_protocols = self.disable_protocols.lock();
            disable_protocols.extend(proto);
        }

        if !no_judges.is_empty() {
            log::warn!("Not found judges for the {:?} protocol. Checking proxy on protocols {:?} is disabled.", no_judges, self.disable_protocols.lock());
        }
        if working == 0 {
            log::error!("Not found judges!");
            exit(0);
        }
        log::info!("{} judges added, Runtime {:?}", working, stime.elapsed());
        self.cv.notify_one();
    }

    pub async fn check_proxy(&mut self, proxy: &mut Proxy) -> bool {
        let expected_types = vec_of_strings!["HTTP", "HTTPS"]; //proxy.expected_types.clone();

        let mut result = vec![];
        for proto in &expected_types {
            result.push(self.check_proto(proxy, proto).await);
        }

        result.iter().any(|i| *i)
    }

    pub async fn check_proto(&mut self, proxy: &mut Proxy, proto: &String) -> bool {
        if self.disable_protocols.lock().contains(proto) {
            return false;
        }

        proxy.negotiator_proto = proto.to_string();
        let judge = self.get_judge(proto);
        proxy.log(format!("Selected judge: {}", judge).as_str(), None, None);

        if proto != "HTTPS" && !proxy.connect().await {
            return false;
        }

        let (negotiate_success, use_full_path, check_anon_lvl) =
            self.negotiate(proxy, &judge, proto).await;
        if !negotiate_success {
            return false;
        }

        let path = judge.url.path().to_string();
        let (raw_request, headers, rv) =
            self.build_raw_request(&judge.host, &path, use_full_path, None);
        let mut is_working = false;

        proxy.send(raw_request.as_bytes()).await;
        if let Some(data) = proxy.recv_all().await {
            proxy.log("Request: success", None, None);
            let mut anonimity_lvl = None;
            let response = Response::parse(data.as_slice());

            if self.is_response_correct(&response, headers, rv) {
                //log::warn!("=====\n{raw_request}\n{0}\n{1}", response.raw, proto);
                if check_anon_lvl {
                    anonimity_lvl = Some(self.get_anonimity_level(&response, judge.marks));
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

    fn type_passed(&self, proxy: &Proxy) {
        unimplemented!();
    }

    async fn in_dnsbl(&self, proxy: &Proxy) {
        unimplemented!();
    }

    async fn negotiate(
        &self,
        proxy: &mut Proxy,
        judge: &Judge,
        proto: &String,
    ) -> (bool, bool, bool) {
        if proto == "SOCKS4" {
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
        } else {
            let negotiator = HttpNegotiator::default();
            (
                negotiator.negotiate().await,
                negotiator.use_full_path,
                negotiator.check_anon_lvl,
            )
        }
    }

    fn get_anonimity_level(&self, response: &Response, marks: BTreeMap<String, usize>) -> String {
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

    fn is_response_correct(
        &self,
        response: &Response,
        headers: BTreeMap<String, String>,
        rv: String,
    ) -> bool {
        let response_raw = response.raw.to_lowercase();
        let version_is_correct = response_raw.contains(&rv);
        let support_referer = if let Some(referer) = headers.get("Referer") {
            response_raw.contains(&referer.to_lowercase())
        } else {
            false
        };
        let support_cookie = if let Some(cookie) = headers.get("Cookie") {
            response_raw.contains(&cookie.to_lowercase())
        } else {
            false
        };
        let some_ip = self.ip_re.find(&response_raw).is_some();
        let is_ok = response.status_code.unwrap_or(0) == 200;

        //println!("{response:#?}: is_ok: {is_ok}, some_ip: {some_ip}, support_cookie: {support_cookie}, support_referer: {support_referer}, rv: {version_is_correct}, {proxy}");
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

        let mut judges = self.judges.lock();
        while !judges.contains_key(&scheme) {
            self.cv.wait(&mut judges)
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
            judges: Arc::new(Mutex::new(BTreeMap::new())),
            disable_protocols: Arc::new(Mutex::new(vec![])),
            ip_re: Regex::new(r#"\d+\.\d+\.\d+\.\d+"#).unwrap(),
            ext_ip: resolver.get_real_ext_ip().await.unwrap(),
            cv: Arc::new(Condvar::new()),
        }
    }
}
