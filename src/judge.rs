use hyper::{client::HttpConnector, Body, Client, Request, StatusCode};
use hyper_tls::HttpsConnector;
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

    pub async fn check_host(&mut self, real_ext_ip: &str) -> bool {
        if self.scheme.to_uppercase().eq("SMTP") {
            self.is_working = true;
        } else {
            let resolver = Resolver::new();
            let c_host = self.url.host_str().unwrap().to_string();
            let ip_address = resolver.resolve(c_host).await;

            if !resolver.host_is_ip(ip_address.as_str()) {
                return false;
            }

            self.ip_address = Some(ip_address);

            // custom connector
            let connector = hyper_tls::native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(!self.verify_ssl)
                .danger_accept_invalid_hostnames(!self.verify_ssl)
                .build()
                .map(|tls| {
                    let mut http = HttpConnector::new();
                    http.enforce_http(false);
                    HttpsConnector::from((http, tls.into()))
                })
                .unwrap();

            let client = Client::builder().build::<_, Body>(connector);
            let request = Request::builder()
                .uri(self.url.to_string())
                .header("User-Agent", random_useragent(true))
                .body(Body::empty())
                .unwrap();

            let task = timeout(
                Duration::from_secs(self.timeout as u64),
                client.request(request),
            );

            match task.await {
                Ok(task_ok) => match task_ok {
                    Ok(response) => {
                        if StatusCode::OK == response.status() {
                            if let Ok(body) = hyper::body::to_bytes(response.into_body()).await {
                                let body_str = String::from_utf8_lossy(&body);
                                self.is_working = body_str
                                    .to_lowercase()
                                    .contains(&real_ext_ip.to_lowercase());
                                self.marks
                                    .insert("via".into(), body_str.matches("via").count());
                                self.marks
                                    .insert("proxy".into(), body_str.matches("proxy").count());
                            }
                        }
                    }
                    Err(err) => log::error!("{}: Error: {}", self, err),
                },
                Err(_) => log::error!("{}: Timeout error", self),
            };
        }

        if self.is_working {
            log::debug!("{}: is working", self);
        } else {
            log::debug!("{}: is not working", self)
        }
        self.is_working
    }
}

// Struct representation
impl std::fmt::Display for Judge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Judge [{}] {}>", self.scheme, self.host)
    }
}

pub fn get_judges() -> Vec<Judge> {
    let mut judges = vec![];
    for url_judge in [
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
        "http://www.proxyjudge.info",
        "http://pascal.hoez.free.fr/azenv.php",
    ] {
        let judge = Judge::new(url_judge);
        judges.push(judge)
    }
    judges
}
