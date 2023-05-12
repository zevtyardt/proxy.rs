pub mod base_provider;
pub mod freeproxylist;
pub mod ipaddress_com;
pub mod proxyscrape;

use lazy_static::lazy_static;

use crate::{proxy::Proxy, utils::queue::FifoQueue};
lazy_static! {
    pub static ref PROXIES: FifoQueue<Proxy> = FifoQueue::new();
}
