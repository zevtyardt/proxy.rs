#![allow(dead_code)]
#![allow(unused_variables)]
//#![allow(unused_imports)]
#![allow(unreachable_code)]

use clap::Parser;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use simple_logger::SimpleLogger;
use std::{sync::Arc, time::Duration};
use tokio::time;

use crate::utils::run_parallel;

mod argument;
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
    let cli = argument::Cli::parse();
    //std::process::exit(0);

    let _ = SimpleLogger::new()
        .with_level(log::LevelFilter::Off)
        .with_module_level("proxy_rs", log::LevelFilter::Info)
        .without_timestamps()
        .init();
    log::info!("Start collecting proxies..");

    RUNTIME.block_on(async move {
        let mut tasks = vec![];

        let max_tries = cli.max_tries as i32;
        let max_conn = cli.max_conn as usize;
        let timeout = cli.timeout as i32;

        let limit = cli.limit.unwrap_or(999999);
        let counter = Arc::new(Mutex::new(1));

        tasks.push(tokio::task::spawn(async move {
            let mut checker = checker::Checker::new().await;
            checker.max_tries = max_tries;
            checker.timeout = timeout;

            checker.check_judges().await;
            loop {
                let mut proxies = vec![];
                while let Ok(mut proxy) = providers::PROXIES.pop() {
                    let mut checker_clone = checker.clone();
                    let counter = counter.clone();
                    let limit = limit;
                    proxies.push(tokio::spawn(async move {
                        if checker_clone.check_proxy(&mut proxy).await {
                            let mut counter = counter.lock();
                            println!("{}", proxy);

                            *counter += 1;
                            if *counter > limit {
                                std::process::exit(0)
                            }
                        }
                    }));
                }

                if !proxies.is_empty() {
                    let stime = time::Instant::now();
                    let t = run_parallel::<()>(proxies, Some(max_conn)).await;

                    log::info!(
                        "Finished checking {} proxies, Runtime {:?}",
                        t.len(),
                        stime.elapsed()
                    )
                }
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
