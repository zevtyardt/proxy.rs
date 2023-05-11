#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use futures_util::{stream, StreamExt};
use lazy_static::lazy_static;
//use std::net::IpAddr;

use crate::{
    //api::ProxyRs,
    providers::freeproxylist::FreeProxyListNetProvider,
    //proxy::Proxy,
    resolver::Resolver,
    utils::{http::random_useragent, CustomFuture},
    //utils::queue::FifoQueue,
};
mod api;
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
    std::env::set_var("RUST_LOG", "proxyrs=debug");
    pretty_env_logger::init();

    RUNTIME.block_on(async {
        let resolver = Resolver::new();

        resolver.resolve("yahoo.com".to_string()).await.unwrap();
        /*
        let ext_ip = resolver.get_real_ext_ip().await.unwrap();
        let my_ip: IpAddr = ext_ip.parse().unwrap();
        resolver.get_ip_info(my_ip).await;

        let queue = FifoQueue::new();

        let proxy = Proxy::new("127.0.0.1".to_string(), 8080).await;
        queue.push(&proxy);
        let proxy2 = Proxy::new("google.com".to_string(), 80).await;
        queue.push(&proxy2);

        log::info!("proxy ip: {}", proxy);
        log::info!("proxy host: {}", proxy2);

        log::info!("{}", queue);

        let mut api = ProxyRs::default();
        api.set_limit(2);
        api.set_timeout(2);
        api.set_max_tries(4);
        api.set_verify_ssl(true);

        println!("{}", api);

        let mut tasks: Vec<CustomFuture<()>> = vec![];

        tasks.push(Box::pin(async { log::debug!("task 1 spawned") }));
        tasks.push(Box::pin(async {
            let mut sub_tasks: Vec<CustomFuture<()>> = vec![];
            sub_tasks.push(Box::pin(async {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                log::debug!("sub tasks 1 spawned");
            }));

            sub_tasks.push(Box::pin(abc()));

            sub_tasks.push(Box::pin(async {
                log::debug!("sub tasks 3 spawned");
            }));

            stream::iter(sub_tasks)
                .buffer_unordered(5)
                .collect::<Vec<()>>()
                .await;
        }));
        tasks.push(Box::pin(async { log::debug!("task 3 spawned") }));
        tasks.push(Box::pin(async { log::debug!("task 4 spawned") }));

        utils::run_parallel::<()>(tasks, 2).await;
        */

        let freeproxylist = FreeProxyListNetProvider::default();
        let proxies = freeproxylist.get_proxies().await;

        log::info!("{:?}", proxies)
    })
}

#[cfg(test)]
mod test;
