use std::{collections::HashMap, time::Duration};

use futures_util::{stream, StreamExt};
use reqwest::Client;
use url::Url;

use crate::resolver::Resolver;

#[derive(Debug)]
pub struct Judge {
    url: Url,

    pub host: String,
    pub scheme: String,
    pub ip_address: Option<String>,
    pub is_working: bool,
    pub marks: HashMap<String, usize>,
    pub timeout: u16,
    pub verify_ssl: bool,
}

impl Judge {
    pub fn new(url: &str) -> Self {
        let url = Url::parse(url).unwrap();
        let mut marks = HashMap::new();
        marks.insert("via".to_string(), 0);
        marks.insert("proxy".to_string(), 0);

        Judge {
            url: url.clone(),
            scheme: url.scheme().to_uppercase(),
            host: url.host_str().unwrap().to_string(),
            ip_address: None,
            is_working: false,
            marks,
            timeout: 8,
            verify_ssl: false,
        }
    }

    pub fn set_verify_ssl(&mut self, value: bool) {
        self.verify_ssl = value
    }

    pub async fn check_host(&mut self, real_ext_ip: &String) -> bool {
        if self.scheme.to_uppercase().eq("SMTP") {
            self.is_working = true;
        } else {
            let resolver = Resolver::new();
            let c_host = self.url.host_str().unwrap().to_string();
            let ip_address = resolver.resolve(c_host).await;

            if !ip_address.is_ok() {
                return false;
            }

            self.ip_address = Some(ip_address.unwrap());

            let client = Client::builder()
                .timeout(Duration::from_secs(self.timeout as u64))
                .danger_accept_invalid_certs(!self.verify_ssl)
                .build()
                .unwrap();
            let request = client.get(self.url.clone()).send().await;
            match request {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        if let Ok(page) = response.text().await {
                            self.is_working =
                                page.to_lowercase().contains(&real_ext_ip.to_lowercase());

                            self.marks
                                .insert("via".to_string(), page.matches("via").count());
                            self.marks
                                .insert("proxy".to_string(), page.matches("proxy").count());
                        }
                    }
                }
                Err(e) => log::error!("{}", e),
            }
        }

        if self.is_working {
            log::info!("{} is working", self);
        } else {
            log::error!("{} is not working", self)
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

pub async fn get_judges(verify_ssl: bool) -> Vec<Judge> {
    let mut fut = vec![];
    let resolver = Resolver::new();
    let real_ext_ip = resolver.get_real_ext_ip().await.unwrap();
    for url_judge in [
        "http://httpheader.net/azenv.php",
        "http://httpbin.org/get?show_env",
        "https://httpbin.org/get?show_env",
        "smtp://smtp.gmail.com",
        "smtp://aspmx.l.google.com",
        "http://azenv.net/",
        "https://www.proxy-listen.de/azenv.php",
        "https://httpheader.net/azenv.php",
        "http://mojeip.net.pl/asdfa/azenv.php",
        "http://proxyjudge.us",
        "http://pascal.hoez.free.fr/azenv.php",
        "http://www.proxy-listen.de/azenv.php",
    ] {
        let mut judge = Judge::new(url_judge);
        let c_real_ext_ip = real_ext_ip.clone();
        let c_verify_ssl = verify_ssl.clone();
        fut.push(async move {
            judge.set_verify_ssl(c_verify_ssl);
            judge.check_host(&c_real_ext_ip).await;
            judge
        })
    }

    stream::iter(fut)
        .buffer_unordered(50)
        .collect::<Vec<Judge>>()
        .await
}
