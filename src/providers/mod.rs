pub mod base_provider;
pub mod freeproxylist;
pub mod git_repo;
pub mod ipaddress_com;
pub mod proxyscan;
pub mod proxyscrape;

use std::sync::{Arc, Mutex};

use concurrent_queue::ConcurrentQueue;
use lazy_static::lazy_static;
use tokio::{spawn, task::JoinHandle};

use crate::proxy::Proxy;

lazy_static! {
    pub static ref PROXIES: ConcurrentQueue<Proxy> = ConcurrentQueue::bounded(500);
    pub static ref UNIQUE_PROXIES: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
}

pub fn get_all_tasks() -> Vec<JoinHandle<()>> {
    let mut tasks = vec![];

    /* ===== */
    tasks.push(spawn(async {
        let mut freeproxylist = freeproxylist::FreeProxyListNetProvider::default();
        freeproxylist.get_proxies().await;
    }));

    /* ===== */
    tasks.push(spawn(async {
        let mut ipaddress_com = ipaddress_com::IpaddressComProvider::default();
        ipaddress_com.get_proxies().await;
    }));

    /* ===== */
    tasks.push(spawn(async {
        let mut proxyscrape_http =
            proxyscrape::proxyscrape_http::ProxyscrapeComHttpProvider::default();
        proxyscrape_http.get_proxies().await;
    }));
    tasks.push(spawn(async {
        let mut proxyscrape_socks4 =
            proxyscrape::proxyscrape_socks4::ProxyscrapeComSocks4Provider::default();
        proxyscrape_socks4.get_proxies().await;
    }));
    tasks.push(spawn(async {
        let mut proxyscrape_socks5 =
            proxyscrape::proxyscrape_socks5::ProxyscrapeComSocks5Provider::default();
        proxyscrape_socks5.get_proxies().await;
    }));

    /* ===== */
    tasks.push(spawn(async {
        let mut zevtyardt_proxy_list =
            git_repo::zevtyardt_proxy_list::GithubZevtyardtProxyListProvider::default();
        zevtyardt_proxy_list.get_proxies().await;
    }));

    /* ===== */
    tasks.push(spawn(async {
        let mut proxyscan_http = proxyscan::proxyscan_http::ProxyscanIoHttpProvider::default();
        proxyscan_http.get_proxies().await;
    }));

    tasks.push(spawn(async {
        let mut proxyscan_https = proxyscan::proxyscan_https::ProxyscanIoHttpsProvider::default();
        proxyscan_https.get_proxies().await;
    }));

    tasks.push(spawn(async {
        let mut proxyscan_socks4 =
            proxyscan::proxyscan_socks4::ProxyscanIoSocks4Provider::default();
        proxyscan_socks4.get_proxies().await;
    }));

    tasks.push(spawn(async {
        let mut proxyscan_socks5 =
            proxyscan::proxyscan_socks5::ProxyscanIoSocks5Provider::default();
        proxyscan_socks5.get_proxies().await;
    }));

    tasks
}
