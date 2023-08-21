use hyper::{client::HttpConnector, Body, Client, Request, StatusCode};
use hyper_tls::HttpsConnector;
use rand::{seq::SliceRandom, thread_rng};
use std::{collections::BTreeMap, time::Duration};
use tokio::time::timeout;
use url::Url;

use crate::{resolver::Resolver, utils::http::random_useragent};

#[derive(Debug, Clone)]
pub struct Judge {
    pub url: Url,

    pub host: String,
    pub scheme: String,
    pub ip_address: Option<String>,
    pub is_working: bool,
    pub marks: BTreeMap<String, usize>,
    pub timeout: u16,
    pub verify_ssl: bool,
}

impl Judge {
    pub fn new(url: &str) -> Self {
        let url = Url::parse(url).unwrap();
        let mut marks = BTreeMap::new();
        marks.insert("via".to_string(), 0);
        marks.insert("proxy".to_string(), 0);

        Judge {
            url: url.clone(),
            scheme: url.scheme().to_uppercase(),
            host: url.host_str().unwrap().to_string(),
            ip_address: None,
            is_working: false,
            marks,
            timeout: 5,
            verify_ssl: false,
        }
    }
}
// Struct representation
impl std::fmt::Display for Judge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Judge [{}] {}>", self.scheme, self.host)
    }
}

pub async fn check_judge_host(judge: &mut Judge, real_ext_ip: &str) -> bool {
    if judge.scheme.to_uppercase().eq("SMTP") {
        judge.is_working = true;
    } else {
        let resolver = Resolver::new();
        let c_host = judge.url.host_str().unwrap().to_string();
        let ip_address = resolver.resolve(c_host).await;

        if !resolver.host_is_ip(ip_address.as_str()) {
            return false;
        }

        judge.ip_address = Some(ip_address);

        // custom connector
        let connector = hyper_tls::native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(!judge.verify_ssl)
            .danger_accept_invalid_hostnames(!judge.verify_ssl)
            .build()
            .map(|tls| {
                let mut http = HttpConnector::new();
                http.enforce_http(false);
                HttpsConnector::from((http, tls.into()))
            })
            .unwrap();

        let client = Client::builder().build::<_, Body>(connector);
        let request = Request::builder()
            .uri(judge.url.to_string())
            .header("User-Agent", random_useragent(true))
            .body(Body::empty())
            .unwrap();

        let task = timeout(
            Duration::from_secs(judge.timeout as u64),
            client.request(request),
        );

        match task.await {
            Ok(task_ok) => match task_ok {
                Ok(response) => {
                    if StatusCode::OK == response.status() {
                        if let Ok(body) = hyper::body::to_bytes(response.into_body()).await {
                            let body_str = String::from_utf8_lossy(&body);
                            judge.is_working = body_str
                                .to_lowercase()
                                .contains(&real_ext_ip.to_lowercase());
                            judge
                                .marks
                                .insert("via".into(), body_str.matches("via").count());
                            judge
                                .marks
                                .insert("proxy".into(), body_str.matches("proxy").count());
                        }
                    }
                }
                Err(err) => log::error!("{}: Error: {}", judge, err),
            },
            Err(_) => log::error!("{}: Timeout error", judge),
        };
    }

    if judge.is_working {
        log::debug!("{}: is working", judge);
    } else {
        log::debug!("{}: is not working", judge)
    }
    judge.is_working
}

pub fn get_judges() -> Vec<Judge> {
    let mut judges = vec![
        "http://httpheader.net/azenv.php",
        "https://httpbin.org/get?show_env",
        "smtp://smtp.gmail.com",
        "http://httpbin.org/get?show_env",
        "https://www.proxy-listen.de/azenv.php",
        "smtp://aspmx.l.google.com",
        "http://azenv.net/",
        "https://httpheader.net/azenv.php",
        "http://mojeip.net.pl/asdfa/azenv.php",
        "http://proxyjudge.us",
        "https://www.proxyjudge.info",
        "http://pascal.hoez.free.fr/azenv.php",
        "http://www.9ravens.com/env.cgi",
        "http://www3.wind.ne.jp/hassii/env.cgi",
        "http://shinh.org/env.cgi",
        "http://www2t.biglobe.ne.jp/~take52/test/env.cgi",
    ]
    .iter()
    .map(|url| Judge::new(url))
    .collect::<Vec<Judge>>();
    judges.shuffle(&mut thread_rng());
    judges
}
