pub mod base_provider;
use std::sync::Arc;

use concurrent_queue::ConcurrentQueue;
use futures_util::{stream, StreamExt};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::{seq::SliceRandom, thread_rng};
use regex::Regex;

use crate::utils::vec_of_strings;

use self::base_provider::Provider;

lazy_static! {
    pub static ref PROXIES: ConcurrentQueue<(String, u16, Vec<String>)> =
        ConcurrentQueue::unbounded();
    pub static ref UNIQUE_PROXIES: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
}

fn update_stack(name: &'static str, proxies: &Vec<(String, u16, Vec<String>)>) {
    let mut added = 0;
    for (ip, port, proto) in proxies {
        let host_port = format!("{}:{}", ip, port);
        let mut unique_proxy = UNIQUE_PROXIES.lock();
        if !unique_proxy.contains(&host_port)
            && PROXIES
                .push((ip.to_owned(), *port, proto.to_owned()))
                .is_ok()
        {
            added += 1;
            unique_proxy.push(host_port)
        }
    }
    log::debug!("{} of {} proxies added from {}", added, proxies.len(), name);
}

pub async fn run_all_providers(num_conn: usize) {
    let mut all_providers = [
        Provider {
            name: "free-proxy-list.net",
            url: "https://free-proxy-list.net",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "api.good-proxies.ru",
            url: "https://api.good-proxies.ru/getfree.php?count=1000&key=freeproxy",
            ..Default::default()
        },
        Provider {
            name: "ipaddress.com",
            url: "https://www.ipaddress.com/proxy-list",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "megaproxylist.net",
            url: "https://www.megaproxylist.net/",
            ..Default::default()
        },
        Provider {
            name: "premiumproxy.net",
            url: "https://premiumproxy.net/full-proxy-list",
            pattern: r#"<font.*?>\s*(?P<ip>(?:\d+\.?){4})\s*<font.*?>\s*\:\s*</font>\s*(?P<port>\d+)"#,
            ..Default::default()
        },
        Provider {
            name: "proxypedia.org",
            url: "https://proxypedia.org/",
            new_urls: Some(|html, host| {
                let mut urls = vec![];
                let re = Regex::new(r#"href="(/free-proxy\/[^\d]+)"#).unwrap();
                for cap in re.captures_iter(html) {
                    let path = cap.get(1).unwrap().as_str();
                    let new_url = format!("{}{}", host, path);
                    urls.push(new_url);
                }
                urls
            }),
            ..Default::default()
        },
        /* proxyscan */
        Provider {
            name: "www.proxyscan.io/..http",
            url: "https://www.proxyscan.io/download?type=http",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "www.proxyscan.io/..https",
            url: "https://www.proxyscan.io/download?type=https",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "www.proxyscan.io/..socks4",
            url: "https://www.proxyscan.io/download?type=socks4",
            proto: vec_of_strings!["SOCKS4"],
            ..Default::default()
        },
        Provider {
            name: "www.proxyscan.io/..socks5",
            url: "https://www.proxyscan.io/download?type=socks5",
            proto: vec_of_strings!["SOCKS5"],
            ..Default::default()
        },
        /* proxyscrape */
        Provider {
            name: "api.proxyscrape.com/..http",
            url: "https://api.proxyscrape.com/?request=getproxies&proxytype=http",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "api.proxyscrape.com/..socks4",
            url: "https://api.proxyscrape.com/?request=getproxies&proxytype=socks4",
            proto: vec_of_strings!["SOCKS4"],
            ..Default::default()
        },
        Provider {
            name: "api.proxyscrape.com/..socks5",
            url: "https://api.proxyscrape.com/?request=getproxies&proxytype=socks5",
            proto: vec_of_strings!["SOCKS5"],
            ..Default::default()
        },
        /* github */
        Provider {
            name: "github.com/zevtyardt/proxy-list",
            url: "https://raw.githubusercontent.com/zevtyardt/proxy-list/main/all.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/TheSpeedX/SOCKS-List/http.txt",
            url: "https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/http.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "github.com/TheSpeedX/SOCKS-List/socks4.txt",
            url: "https://raw.githubusercontentent.com/TheSpeedX/SOCKS-List/master/socks4.txt",
            proto: vec_of_strings!["SOCKS4"],
            ..Default::default()
        },
        Provider {
            name: "github.com/TheSpeedX/SOCKS-List/socks5.txt",
            url: "https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/socks5.txt",
            proto: vec_of_strings!["SOCKS5"],
            ..Default::default()
        },
    ];
    all_providers.shuffle(&mut thread_rng());

    stream::iter(all_providers)
        .map(|f| async move { (f.name, f.get_proxies().await) })
        .buffer_unordered(num_conn)
        .map(|(name, proxies)| update_stack(name, &proxies))
        .collect::<Vec<()>>()
        .await;
}
