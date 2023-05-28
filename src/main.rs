#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use std::time::Duration;

use lazy_static::lazy_static;
use tokio::{spawn, time};

use crate::utils::run_parallel;

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
    if option_env!("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "proxy_rs=warn");
    }
    pretty_env_logger::init();

    RUNTIME.block_on(async {
        log::info!("Start collecting proxies and judges");
        let stime = time::Instant::now();
        let mut tasks = vec![];

        tasks.push(tokio::task::spawn(async {
            let mut checker = checker::Checker::new().await;
            checker.check_judges().await;
            loop {
                let mut tasks = vec![];
                while let Ok(mut proxy) = providers::PROXIES.pop() {
                    let mut checker_clone = checker.clone();
                    tasks.push(spawn(async move {
                        checker_clone.check_proxy(&mut proxy).await;
                    }));
                }

                if !tasks.is_empty() {
                    let stime = time::Instant::now();
                    let len_tasks = tasks.len();

                    run_parallel::<()>(tasks, Some(200)).await;
                    log::info!(
                        "{} proxies checked, Runtime {:?}",
                        len_tasks,
                        stime.elapsed()
                    );
                }
                time::sleep(Duration::from_secs(5)).await;
            }
        }));

        /* providers */
        tasks.push(tokio::task::spawn(async {
            loop {
                let all_providers = providers::get_all_tasks();
                run_parallel::<()>(all_providers, None).await;
                time::sleep(Duration::from_secs(10)).await;
            }
        }));

        run_parallel::<()>(tasks, None).await;
    });
}
