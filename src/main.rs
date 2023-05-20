#![allow(dead_code)]
#![allow(unused_variables)]
//#![allow(unused_imports)]

use futures_util::{stream, StreamExt};
use indicatif::HumanDuration;
use lazy_static::lazy_static;
use tokio::time;

//mod api;
mod checker;
mod judge;
mod negotiators;
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
    std::env::set_var("RUST_LOG", "proxy_rs=info");
    pretty_env_logger::init();

    RUNTIME.block_on(async {
        log::info!("Start collecting proxies and judges");
        let stime = time::Instant::now();
        let mut tasks = providers::get_all_tasks();
        let checker = checker::Checker::new().await;

        let mut checker_c = checker.clone();
        tasks.push(tokio::task::spawn(async move {
            checker_c.check_judges().await;
        }));

        stream::iter(tasks)
            .map(|f| async { f.await.unwrap() })
            .buffered(20)
            .collect::<Vec<()>>()
            .await;

        let total_proxies = providers::PROXIES.qsize();
        log::info!(
            "{} proxies collected, Runtime {:?}",
            total_proxies,
            stime.elapsed()
        );

        let mut proxies = vec![];
        while let Some(proxy) = providers::PROXIES.get_nowait() {
            proxies.push(proxy);
        }

        let s = stream::iter(proxies)
            .map(|mut proxy| {
                let mut checker_cc = checker.clone();
                async move {
                    if checker_cc.check(&mut proxy).await {
                        println!("{}", proxy)
                    }
                }
            })
            .buffer_unordered(500);
        s.collect::<Vec<()>>().await;

        log::info!(
            "{} Proxy checked, Runtime {}",
            total_proxies,
            HumanDuration(stime.elapsed())
        );
    });
}
