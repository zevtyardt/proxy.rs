pub mod base_provider;

use std::sync::Arc;

use concurrent_queue::ConcurrentQueue;
use dashmap::DashSet;
use futures_util::{stream::FuturesUnordered, StreamExt};
use lazy_static::lazy_static;
use rand::{seq::SliceRandom, thread_rng};
use regex::Regex;
use tokio::sync::Semaphore;

use crate::{proxy::Proxy, utils::vec_of_strings};

use self::base_provider::{Provider, ProviderTask};

lazy_static! {
    pub static ref PROXIES: ConcurrentQueue<Proxy> = ConcurrentQueue::unbounded();
    pub static ref UNIQUE_PROXIES: DashSet<String> = DashSet::new();
}

pub fn providers() -> Vec<Provider> {
    let s = std::time::Instant::now();
    let mut providers = vec![
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
        Provider {
            name: "openproxylist.xyz",
            url: "https://openproxylist.xyz/http.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        /* proxyspace.pro */
        Provider {
            name: "proxyspace.pro/http.txt",
            url: "https://proxyspace.pro/http.txt",
            ..Default::default()
        },
        Provider {
            name: "proxyspace.pro/https.txt",
            url: "https://proxyspace.pro/https.txt",
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
            url: "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/blob/master/socks4.txt",
            proto: vec_of_strings!["SOCKS4"],
            ..Default::default()
        },
        Provider {
            name: "github.com/TheSpeedX/SOCKS-List/socks5.txt",
            url: "https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/socks5.txt",
            proto: vec_of_strings!["SOCKS5"],
            ..Default::default()
        },
        Provider {
            name: "github.com/almroot/proxylist",
            url: "https://raw.githubusercontent.com/almroot/proxylist/master/list.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/andigwandi/free-proxy",
            url: "https://raw.githubusercontent.com/andigwandi/free-proxy/main/proxy_list.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/aslisk/proxyhttps",
            url: "https://raw.githubusercontent.com/aslisk/proxyhttps/main/https.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/B4RC0DE-TM/proxy-list",
            url: "https://raw.githubusercontent.com/B4RC0DE-TM/proxy-list/main/HTTP.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/monosans/proxy-list/http.txt",
            url: "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/monosans/proxy-list/socks4.txt",
            url: "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks4.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/monosans/proxy-list/socks5.txt",
            url: "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks5.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/fahimscirex/proxybd/master/proxylist/http.txt",
            url: "https://raw.githubusercontent.com/fahimscirex/proxybd/master/proxylist/http.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "github.com/hendrikbgr/Free-Proxy-Repo",
            url:
                "https://raw.githubusercontent.com/hendrikbgr/Free-Proxy-Repo/master/proxy_list.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/HyperBeats/proxy-list",
            url: "https://raw.githubusercontent.com/HyperBeats/proxy-list/main/http.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "github.com/jetkai/proxy-list/http.txt",
            url: "https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-http.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "github.com/jetkai/proxy-list/https.txt",
            url: "https://raw.githubusercontent.com/jetkai/proxy-list/main/online-proxies/txt/proxies-https.txt",
            ..Default::default()
        },
        Provider {
            name: "github.com/mertguvencli/http-proxy-list",
            url: "https://raw.githubusercontent.com/mertguvencli/http-proxy-list/main/proxy-list/data.txt",
            ..Default::default()
        },
         Provider {
            name: "github.com/miyukii-chan/proxy-list",
            url: "https://raw.githubusercontent.com/miyukii-chan/proxy-list/master/proxies/http.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "github.com/mmpx12/proxy-list/http.txt",
            url: "https://raw.githubusercontent.com/mmpx12/proxy-list/master/http.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
        Provider {
            name: "github.com/mmpx12/proxy-list/https.txt",
            url: "https://raw.githubusercontent.com/mmpx12/proxy-list/master/https.txt",
            proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
            ..Default::default()
        },
    ];

    providers.shuffle(&mut thread_rng());

    /* DEBUGGING CODE

    providers = vec![];

    if !providers.is_empty() {
        providers.sort_by(|a, b| a.url.cmp(&b.url));
    }
    for p in &providers {
        println!("  - [x] `{}`", p.url);
    }

    println!("\n  **total sources: {}**", providers.len());

    */

    log::info!(
        "loaded {} providers. Runtime {:?}",
        providers.len(),
        s.elapsed()
    );
    providers
}

async fn update_stack(name: &'static str, proxies: &Vec<(String, u16, Vec<String>)>) {
    let mut added = 0;
    for (ip, port, proto) in proxies {
        let host_port = format!("{}:{}", ip, port);
        if UNIQUE_PROXIES.get(&host_port).is_some() {
            continue;
        }

        if let Some(proxy) = Proxy::create(ip, *port, proto.to_vec()).await {
            if PROXIES.push(proxy).is_ok() {
                added += 1;
                UNIQUE_PROXIES.insert(host_port);
            };
        }
    }
    log::debug!("{} of {} proxies added from {}", added, proxies.len(), name);
}

pub async fn run_all_providers(num_conn: usize) {
    let sem = Arc::new(Semaphore::new(num_conn));
    let mut futures = FuturesUnordered::new();
    for provider in providers() {
        let permit = Arc::clone(&sem).acquire_owned().await;
        futures.push(tokio::spawn(async move {
            let _ = permit;
            let name = provider.name;
            let task = ProviderTask::new(provider);
            let proxies = task.get_proxies().await;
            update_stack(name, &proxies).await;
        }));
    }

    while (futures.next().await).is_some() {
        continue;
    }
}
