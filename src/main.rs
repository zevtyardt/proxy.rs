#![allow(dead_code)]
//#![allow(unused_variables)]
//#![allow(unused_imports)]

use futures_util::future::join_all;
use lazy_static::lazy_static;

//mod api;
mod judge;
mod providers;
mod proxy;
mod resolver;
mod utils;

lazy_static! {
    pub static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
}

fn main() {
    std::env::set_var("RUST_LOG", "proxy_rs=debug");
    pretty_env_logger::init();

    RUNTIME.block_on(async {
        join_all(providers::get_all_tasks()).await;
        log::info!("Total proxies scraped: {}", providers::PROXIES.qsize(),);

        let mut dupe = vec![];
        let data = providers::PROXIES.data.lock().unwrap();
        for prox in data.clone().into_iter() {
            if dupe.contains(&prox) {
                log::info!("Duplicate {}", prox);
            }
            dupe.push(prox)
        }
    })
}
