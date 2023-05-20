use std::{
    collections::HashMap,
    process::exit,
    sync::{Arc, Condvar, Mutex},
};

use rand::seq::SliceRandom;
use regex::Regex;
use tokio::time;

use crate::{
    judge::{get_judges, Judge},
    negotiators::http_negotiator::HttpNegotiator,
    proxy::Proxy,
    resolver::Resolver,
    utils::{
        http::{get_headers, Response},
        vec_of_strings,
    },
};

#[derive(Clone)]
pub struct Checker {
    pub verify_ssl: bool,
    pub timeout: i32,
    pub max_tries: i32,
    pub method: String,

    judges: Arc<Mutex<HashMap<String, Vec<Judge>>>>,
    ext_ip: String,
    ip_re: Regex,
    cv: Arc<Condvar>,
}

impl Checker {
    pub async fn check_judges(&mut self) {
        let stime = time::Instant::now();

        let mut works = 0;
        for judge in get_judges(self.verify_ssl).await {
            if judge.is_working {
                let scheme = &judge.scheme;
                let mut judges = self.judges.lock().unwrap();
                if !judges.contains_key(scheme) {
                    judges.insert(scheme.clone(), vec![]);
                }

                let v = judges.get_mut(scheme).unwrap();
                v.push(judge.clone());
                works += 1;
            }
        }

        let mut nojudges = vec![];
        let mut disable_protocols = vec![];

        for (scheme, proto) in [
            (
                "HTTP".to_string(),
                vec_of_strings!["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"],
            ),
            ("HTTPS".to_string(), vec_of_strings!["HTTPS"]),
            ("SMTP".to_string(), vec_of_strings!["SMTP"]),
        ] {
            if self.judges.lock().unwrap().get(&scheme).unwrap().is_empty() {
                nojudges.push(scheme.clone());
                disable_protocols.extend(proto);
            }
        }

        if !nojudges.is_empty() {
            log::warn!("Not found judges for the {:?} protocol. Checking proxy on protocols {:?} is disabled.", nojudges, disable_protocols);
        }

        if works > 0 {
            log::debug!("{} judges added, Runtime {:?}", works, stime.elapsed());
        } else {
            log::error!("Not found judges!");
            exit(0)
        }
        self.cv.notify_one();
    }

    pub async fn type_passed(&self, proxy: &Proxy) {
        unimplemented!();
    }

    pub async fn in_dnsbl(&self, proxy: &Proxy) {
        unimplemented!();
    }

    pub async fn check(&mut self, proxy: &mut Proxy) -> bool {
        let expected_types = proxy.expected_types.clone();
        for proto in expected_types.into_iter() {
            proxy.negotiator_proto = proto.to_string();

            let judge = self.get_judge(&proto);
            if proxy.connect().await {
                // http is default, TODO: add another protos
                let negotiator = HttpNegotiator::default();
                let negotiate_success = negotiator
                    .negotiate(&judge.host, &judge.ip_address.unwrap())
                    .await;

                if negotiate_success {
                    let (raw_request, headers, rv) = self.build_raw_request(
                        &judge.url.scheme().to_string(),
                        &judge.host,
                        &judge.url.path().to_string(),
                        None,
                    );
                    proxy.send(raw_request.as_bytes()).await;
                    if let Some(data) = proxy.recv_all().await {
                        let response = Response::parse(data.as_slice());
                        proxy.log("Request: success", None, None);

                        if self.is_response_correct(&response, headers, rv) {
                            proxy.log("Response: correct", None, None);
                            let anonimity_lvl = self.get_anonimity_level(&response, judge.marks);
                            proxy.types.push((proto, Some(anonimity_lvl)));
                            return true;
                        } else {
                            proxy.log(
                                "Response: not correct",
                                None,
                                Some("response_not_correct".to_string()),
                            )
                        }
                    } else {
                        proxy.log("Request: failed", None, Some("request_failed".to_string()));
                    }
                }
            }
            break;
        }
        false
    }

    fn get_anonimity_level(&self, response: &Response, marks: HashMap<String, usize>) -> String {
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
            "Elite".to_string()
        }
    }

    fn is_response_correct(
        &self,
        response: &Response,
        headers: HashMap<String, String>,
        rv: String,
    ) -> bool {
        let response_raw = &response.raw;
        let version_is_correct = response_raw.contains(&rv);
        let support_referer = if let Some(referer) = headers.get("Referer") {
            response_raw.contains(referer)
        } else {
            false
        };
        let support_cookie = if let Some(cookie) = headers.get("Cookie") {
            response_raw.contains(cookie)
        } else {
            false
        };
        let some_ip = self.ip_re.find(&response_raw);
        let is_ok = response.status_code.unwrap_or(0) == 200;
        is_ok && version_is_correct && support_referer && support_cookie && some_ip.is_some()
    }

    fn build_raw_request(
        &self,
        scheme: &String,
        host: &String,
        path: &String,
        data: Option<String>,
    ) -> (String, HashMap<String, String>, String) {
        let mut request = format!("{} {}://{}{} HTTP/1.1\r\n", self.method, scheme, host, path);
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

    fn get_judge(&mut self, proto: &String) -> Judge {
        let mut scheme = "HTTP".to_string();
        let proto = proto.as_str();
        if proto.eq("HTTPS") {
            scheme = "HTTPS".to_string();
        } else if proto.eq("CONNECT:25") {
            scheme = "SMTP".to_string();
        }

        let mut judges = self.judges.lock().unwrap();
        while judges.is_empty() {
            judges = self.cv.wait(judges).unwrap();
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
            judges: Arc::new(Mutex::new(HashMap::new())),
            ip_re: Regex::new(r#"\d+\.\d+\.\d+\.\d+"#).unwrap(),
            ext_ip: resolver.get_real_ext_ip().await.unwrap(),
            cv: Arc::new(Condvar::new()),
        }
    }
}
