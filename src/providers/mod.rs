pub mod base_provider;
pub mod freeproxylist;
pub mod github;
pub mod ipaddress_com;
pub mod premiumproxy_net;
pub mod proxyscan;
pub mod proxyscrape;

use std::sync::Arc;

use concurrent_queue::ConcurrentQueue;
use futures_util::{stream, StreamExt};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::seq::SliceRandom;

lazy_static! {
    pub static ref PROXIES: ConcurrentQueue<(String, u16, Vec<String>)> =
        ConcurrentQueue::unbounded();
    pub static ref UNIQUE_PROXIES: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
}
pub async fn run_all_providers(num_conn: usize) {
    let num_providers: u32 = 14;

    let mut numbers = (0..num_providers).collect::<Vec<u32>>();
    numbers.shuffle(&mut rand::thread_rng());

    stream::iter(numbers)
        .map(|n| async move {
            match n {
                0 => {
                    let mut freeproxylist = freeproxylist::FreeProxyListNetProvider::default();
                    freeproxylist.get_proxies().await;
                }
                1 => {
                    let mut ipaddress_com = ipaddress_com::IpaddressComProvider::default();
                    ipaddress_com.get_proxies().await;
                }
                2 => {
                    let mut proxyscrape_http =
                        proxyscrape::http::ProxyscrapeComHttpProvider::default();
                    proxyscrape_http.get_proxies().await;
                }
                3 => {
                    let mut proxyscrape_socks4 =
                        proxyscrape::socks4::ProxyscrapeComSocks4Provider::default();
                    proxyscrape_socks4.get_proxies().await;
                }
                4 => {
                    let mut proxyscrape_socks5 =
                        proxyscrape::socks5::ProxyscrapeComSocks5Provider::default();
                    proxyscrape_socks5.get_proxies().await;
                }
                5 => {
                    let mut zevtyardt_proxy_list =
                        github::zevtyardt_proxy_list::GithubZevtyardtProxyListProvider::default();
                    zevtyardt_proxy_list.get_proxies().await;
                }
                6 => {
                    let mut thespeedx_http_list =
                        github::thespeedx_socks_list::http::GithubTheSpeedXProxyListHttpProvider::default();
                    thespeedx_http_list.get_proxies().await;
                }
                7 => {
                    let mut thespeedx_socks4_list =
                        github::thespeedx_socks_list::socks4::GithubTheSpeedXProxyListSocks4Provider::default();
                    thespeedx_socks4_list.get_proxies().await;
                }
                8 => {
                    let mut thespeedx_socks5_list =
                        github::thespeedx_socks_list::socks5::GithubTheSpeedXProxyListSocks5Provider::default();
                    thespeedx_socks5_list.get_proxies().await;
                }
                9 => {
                    let mut proxyscan_http = proxyscan::http::ProxyscanIoHttpProvider::default();
                    proxyscan_http.get_proxies().await;
                }
                10 => {
                    let mut proxyscan_https = proxyscan::https::ProxyscanIoHttpsProvider::default();
                    proxyscan_https.get_proxies().await;
                }
                11 => {
                    let mut proxyscan_socks4 =
                        proxyscan::socks4::ProxyscanIoSocks4Provider::default();
                    proxyscan_socks4.get_proxies().await;
                }
                12 => {
                    let mut proxyscan_socks5 =
                        proxyscan::socks5::ProxyscanIoSocks5Provider::default();
                    proxyscan_socks5.get_proxies().await;
                }
                13 => {
                    let mut premiumproxy_net = premiumproxy_net::PremiumproxyNetProvider::default();
                    premiumproxy_net.get_proxies().await;
                }
                _ => {}
            }
        })
        .buffer_unordered(num_conn)
        .collect::<Vec<()>>()
        .await;
}
