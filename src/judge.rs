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
        }
    }

    pub async fn check_host(&mut self, real_ext_ip: &String) -> bool {
        if self.scheme.to_uppercase().eq("SMTP") {
            self.is_working = true;
        } else {
            let resolver = Resolver::new();
            let c_host = self.url.host_str().unwrap().to_string();
            let ip_address = tokio::task::spawn_blocking(move || resolver.resolve(c_host))
                .await
                .unwrap();

            if ip_address.is_none() {
                return false;
            }

            self.ip_address = ip_address;

            let client = Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();
            let request = client.get(self.url.clone()).send().await;
            match request {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        if let Ok(page) = response.text().await {
                            self.is_working = page.to_lowercase().contains(real_ext_ip);

                            self.marks
                                .insert("via".to_string(), page.matches("via").count());
                            self.marks
                                .insert("proxy".to_string(), page.matches("proxy").count());
                        }
                    }
                }
                Err(e) => eprintln!("Err: {}", e),
            }
        }

        if self.is_working {
            println!("Info: {} is working", self);
        } else {
            println!("Info: {} is not working", self)
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

pub async fn get_judges() -> Vec<Judge> {
    let mut fut = vec![];
    let resolver = Resolver::new();
    let real_ext_ip = resolver.get_real_ext_ip().await.unwrap();
    for url_judge in [
        "http://httpbin.org/get?show_env",
        "https://httpbin.org/get?show_env",
        "smtp://smtp.gmail.com",
        "smtp://aspmx.l.google.com",
        "http://azenv.net/",
        "https://www.proxy-listen.de/azenv.php",
        "http://www.proxyfire.net/fastenv",
        "http://proxyjudge.us/azenv.php",
        "http://ip.spys.ru/",
        "http://www.proxy-listen.de/azenv.php",
    ] {
        let mut judge = Judge::new(url_judge);
        let c_real_ext_ip = real_ext_ip.clone();
        fut.push(async move {
            judge.check_host(&c_real_ext_ip).await;
            judge
        })
    }

    stream::iter(fut)
        .buffer_unordered(10)
        .collect::<Vec<Judge>>()
        .await
}
