#![allow(dead_code)]
#![allow(unused_variables)]
//#![allow(unused_imports)]
#![allow(unreachable_code)]

use clap::Parser;
use parking_lot::Mutex;
use simple_logger::SimpleLogger;
use std::{sync::Arc, time::Duration};
use tokio::time;

use crate::{argument::Commands, utils::run_parallel};

mod argument;
mod checker;
mod judge;
mod negotiators;
mod providers;
mod proxy;
mod resolver;
mod utils;

fn main() {
    let cli = argument::Cli::parse();

    let log_level = match cli.log_level.as_str() {
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Warn,
    };
    let _ = SimpleLogger::new()
        .with_level(log::LevelFilter::Off)
        .with_module_level("proxy_rs", log_level)
        .without_timestamps()
        .init();

    log::info!("Start collecting proxies.. ",);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let mut tasks = vec![];

            let max_tries = cli.max_tries as i32;
            let max_conn = cli.max_conn as usize;
            let timeout = cli.timeout as i32;
            let counter = Arc::new(Mutex::new(1));

            match cli.sub {
                Commands::Grab(grab_args) => {
                    let expected_countries = grab_args.countries;
                    let format = grab_args.format;
                    let limit = grab_args.limit.unwrap_or(999999);

                    tasks.push(tokio::task::spawn(async move {
                        if format == "json" {
                            print!("[")
                        }

                        loop {
                            while let Ok(proxy) = providers::PROXIES.pop() {
                                let mut counter = counter.lock();
                                *counter += 1;

                                match format.as_str() {
                                    "text" => print!("{}", proxy.as_text()),
                                    "json" => print!("{}", proxy.as_json()),
                                    _ => print!("{}", proxy),
                                }
                                if *counter > limit {
                                    if format == "json" {
                                        println!("]");
                                    } else {
                                        println!()
                                    }
                                    std::process::exit(0)
                                } else if format == "json" {
                                    print!(",");
                                }
                                println!()
                            }
                        }
                    }))
                }

                Commands::Find(find_args) => {
                    let expected_types = find_args.types;
                    let expected_levels = find_args.levels;
                    let expected_countries = find_args.countries;
                    let format = find_args.format;
                    let limit = find_args.limit.unwrap_or(999999);
                    let verify_ssl = false;

                    let mut checker = checker::Checker::new().await;
                    let ext_ip = checker.ext_ip.clone();

                    tasks.push(tokio::task::spawn(async move {
                        checker::check_judges(verify_ssl, ext_ip).await;
                    }));

                    tasks.push(tokio::task::spawn(async move {
                        checker.max_tries = max_tries;
                        checker.timeout = timeout;
                        checker.expected_types = expected_types;
                        checker.expected_levels = expected_levels;
                        checker.expected_countries = expected_countries;

                        if format == "json" {
                            print!("[")
                        }
                        loop {
                            let mut proxies = Vec::with_capacity(1000);
                            while let Ok(mut proxy) = providers::PROXIES.pop() {
                                if proxies.len() >= 1000 {
                                    break;
                                }
                                let mut checker_clone = checker.clone();
                                let counter = counter.clone();
                                let limit = limit;
                                let format = format.clone();
                                proxies.push(tokio::task::spawn(async move {
                                    if checker_clone.check_proxy(&mut proxy).await {
                                        let mut counter = counter.lock();
                                        *counter += 1;

                                        match format.as_str() {
                                            "text" => print!("{}", proxy.as_text()),
                                            "json" => print!("{}", proxy.as_json()),
                                            _ => print!("{}", proxy),
                                        }
                                        if *counter > limit {
                                            if format == "json" {
                                                println!("]");
                                            } else {
                                                println!()
                                            }
                                            std::process::exit(0)
                                        } else if format == "json" {
                                            print!(",");
                                        }
                                        println!()
                                    }
                                }));
                            }

                            if !proxies.is_empty() {
                                let stime = time::Instant::now();
                                let ret = run_parallel::<()>(proxies, Some(max_conn)).await;

                                log::info!(
                                    "Finished checking {} proxies. Runtime {:?}",
                                    ret.len(),
                                    stime.elapsed()
                                )
                            }
                        }
                    }));
                }
            }

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
