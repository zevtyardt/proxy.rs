use std::{collections::BTreeMap, time::Duration};

use reqwest::Client;
use url::Url;

use crate::resolver::Resolver;

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
            timeout: 8,
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
                Err(e) => log::debug!("{}", e),
            }
        }

        if self.is_working {
            log::debug!("{} is working", self);
        } else {
            log::debug!("{} is not working", self)
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
        "http://pascal.hoez.free.fr/azenv.php",
    ] {
        let judge = Judge::new(url_judge);
        judges.push(judge)
    }
    judges
}
