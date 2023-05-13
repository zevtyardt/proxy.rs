pub mod base_provider;
pub mod freeproxylist;
pub mod ipaddress_com;
pub mod proxyscrape;

use lazy_static::lazy_static;
use tokio::task::{self, JoinHandle};

use crate::{proxy::Proxy, utils::queue::FifoQueue};
lazy_static! {
    pub static ref PROXIES: FifoQueue<Proxy> = FifoQueue::new();
}

pub fn get_all_tasks() -> Vec<JoinHandle<()>> {
    let mut tasks = vec![];

    /* ===== */
    tasks.push(task::spawn(async {
        let mut freeproxylist = freeproxylist::FreeProxyListNetProvider::default();
        freeproxylist.get_proxies().await;
    }));

    /* ===== */
    tasks.push(task::spawn(async {
        let mut ipaddress_com = ipaddress_com::IpaddressComProvider::default();
        ipaddress_com.get_proxies().await;
    }));

    /* ===== */
    tasks.push(task::spawn(async {
        let mut proxyscrape_http =
            proxyscrape::proxyscrape_http::ProxyscrapeComHttpProvider::default();
        proxyscrape_http.get_proxies().await;
    }));
    tasks.push(task::spawn(async {
        let mut proxyscrape_socks4 =
            proxyscrape::proxyscrape_socks4::ProxyscrapeComSocks4Provider::default();
        proxyscrape_socks4.get_proxies().await;
    }));
    tasks.push(task::spawn(async {
        let mut proxyscrape_socks5 =
            proxyscrape::proxyscrape_socks5::ProxyscrapeComSocks5Provider::default();
        proxyscrape_socks5.get_proxies().await;
    }));

    tasks
}
