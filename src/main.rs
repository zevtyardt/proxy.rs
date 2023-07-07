#![allow(dead_code)]
//#![allow(unused_variables)]
//#![allow(unused_imports)]
//#![allow(unreachable_code)]

use argument::GrabArgs;
use checker::Checker;
use clap::Parser;
use futures_util::{stream, StreamExt};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use proxy::Proxy;
use regex::Regex;
use server::{proxy_pool::LIVE_PROXIES, Server};
use simple_logger::SimpleLogger;
use std::{path::PathBuf, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    fs::File,
    io::{stdout, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    runtime,
    sync::mpsc::{self, Sender},
    task,
    time::{self, interval},
};

use crate::{
    argument::{Cli, Commands},
    providers::PROXIES,
    utils::update::check_version,
};

mod argument;
mod checker;
mod judge;
mod negotiators;
mod providers;
mod proxy;
mod resolver;
mod server;
mod utils;

lazy_static! {
    static ref STOP_FIND_LOOP: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

async fn handle_grab_command(args: GrabArgs, tx: Sender<Option<Proxy>>) {
    let expected_countries = args.countries;

    loop {
        if let Ok((host, port, expected_types)) = PROXIES.pop() {
            if let Some(proxy) = proxy::Proxy::create(host.as_str(), port, expected_types).await {
                if !expected_countries.is_empty()
                    && !expected_countries.contains(&proxy.geo.iso_code)
                {
                    continue;
                }
                if tx.send(Some(proxy)).await.is_err() {
                    return;
                }
            }
        }
    }
}

async fn handle_find_command(checker: Checker, max_conn: usize, tx: Sender<Option<Proxy>>) {
    while !*STOP_FIND_LOOP.lock() {
        let mut proxies = Vec::new();
        while let Ok((host, port, expected_types)) = PROXIES.pop() {
            if let Some(proxy) = Proxy::create(host.as_str(), port, expected_types).await {
                proxies.push(proxy)
            }
        }

        if !proxies.is_empty() {
            let stime = time::Instant::now();
            let ret = stream::iter(proxies)
                .map(|mut proxy| {
                    let mut checker = checker.clone();
                    let tx = tx.clone();
                    task::spawn(async move {
                        if checker.check_proxy(&mut proxy).await {
                            tx.send(Some(proxy)).await.unwrap();
                        }
                    })
                })
                .map(|f| async { f.await.unwrap_or(()) })
                .buffer_unordered(max_conn)
                .collect::<Vec<()>>()
                .await;

            log::info!(
                "Finished checking {} proxies. Runtime {:?}",
                ret.len(),
                stime.elapsed()
            );

            if *STOP_FIND_LOOP.lock() {
                tx.send(None).await.unwrap();
            }
        }
    }
}

async fn handle_file_input(files: Vec<PathBuf>) {
    for file in files {
        match File::open(&file).await {
            Ok(file) => {
                let ip_port = Regex::new(r#"(?P<ip>(?:\d+\.?){4}):(?P<port>\d+)"#).unwrap();
                let buffer = BufReader::new(file);
                let mut lines = buffer.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(cap) = ip_port.captures(&line) {
                        let ip = cap.get(1).unwrap().as_str();
                        let port = cap.get(2).unwrap().as_str();

                        if let Ok(port) = port.parse::<u16>() {
                            PROXIES.push((ip.to_string(), port, vec![])).unwrap();
                        };
                    }
                }
            }
            Err(e) => log::error!("{}: {:?}", e, file),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let log_level = match cli.log_level.as_str() {
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Warn,
    };

    SimpleLogger::new()
        .with_level(log::LevelFilter::Off)
        .with_module_level("proxy_rs", log_level)
        .without_timestamps()
        .init()
        .unwrap();

    runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let mut tasks = vec![];

            let max_conn = cli.max_conn;
            let timeout = cli.timeout as i32;

            let mut files = vec![];
            let (tx, mut rx) = mpsc::channel(50);

            let mut outfile = None;
            let mut limit = 0;
            let mut format = "default".to_string();

            let mut is_server = false;
            let mut host = "127.0.0.1".to_string();
            let mut port = 8080;

            if !cli.skip_version_check {
                tasks.push(task::spawn(check_version()));
            }

            match cli.sub {
                Commands::Grab(grab_args) => {
                    outfile = grab_args.outfile.clone();
                    limit = grab_args.limit;
                    format = grab_args.format.clone();

                    let tx = tx.clone();
                    tasks.push(task::spawn(handle_grab_command(grab_args, tx)))
                }
                Commands::Find(find_args) => {
                    outfile = find_args.outfile.clone();
                    limit = find_args.limit;
                    format = find_args.format.clone();

                    let mut checker = Checker::new().await;
                    checker.max_tries = find_args.max_tries as i32;
                    checker.timeout = timeout;
                    checker.support_cookie = find_args.support_cookies;
                    checker.support_referer = find_args.support_referer;
                    checker.expected_types = find_args.types.clone();
                    checker.expected_levels = find_args.levels;
                    checker.expected_countries = find_args.countries;

                    let ext_ip = checker.ext_ip.clone();

                    let expected_types = find_args.types.clone();
                    let verify_ssl = false;
                    tasks.push(task::spawn(async move {
                        checker::check_judges(verify_ssl, ext_ip, expected_types).await;
                    }));

                    files.extend(find_args.files.clone());

                    let tx = tx.clone();
                    tasks.push(task::spawn(handle_find_command(checker, max_conn, tx)));
                }

                Commands::Serve(serve_args) => {
                    is_server = true;

                    host = serve_args.host;
                    port = serve_args.port;

                    let mut checker = Checker::new().await;
                    checker.max_tries = serve_args.max_tries as i32;
                    checker.support_cookie = true;
                    checker.support_referer = true;

                    checker.expected_types = serve_args.types.clone();
                    checker.expected_levels = serve_args.levels;
                    checker.expected_countries = serve_args.countries;

                    let ext_ip = checker.ext_ip.clone();

                    let expected_types = serve_args.types.clone();
                    let verify_ssl = false;
                    tasks.push(task::spawn(async move {
                        checker::check_judges(verify_ssl, ext_ip, expected_types).await;
                    }));

                    files.extend(serve_args.files.clone());

                    let tx = tx.clone();
                    tasks.push(task::spawn(handle_find_command(checker, max_conn, tx)));
                }
            }

            if !files.is_empty() {
                tasks.push(task::spawn(async move {
                    handle_file_input(files).await;
                    let mut stop_file_loop = STOP_FIND_LOOP.lock();
                    *stop_file_loop = true
                }))
            } else {
                if !is_server {
                    log::info!("Start collecting proxies.. ");
                }

                /* providers */
                tasks.push(tokio::task::spawn(async {
                    let mut interval = interval(Duration::from_secs(60));
                    loop {
                        interval.tick().await;
                        tokio::task::spawn(providers::run_all_providers(2));
                    }
                }));
            }

            if is_server {
                tasks.push(tokio::task::spawn(async move {
                    let server = Server::new(host.as_str(), port);
                    server.start().await;
                }));

                loop {
                    if let Some(Some(proxy)) = rx.recv().await {
                        while LIVE_PROXIES.is_full() {
                            continue;
                        }
                        LIVE_PROXIES.push(proxy).unwrap();
                    }
                }
            } else {
                let mut output: Pin<Box<dyn AsyncWrite>> = if let Some(path) = outfile {
                    let file = File::create(path).await.unwrap();
                    Box::pin(file)
                } else {
                    Box::pin(stdout())
                };

                let mut open_list = false;
                let mut counter = limit;

                while let Some(proxy) = rx.recv().await {
                    let stop = proxy.is_none() || (limit != 0 && counter <= 1);
                    if let Some(proxy) = proxy {
                        if format == "json" && !open_list {
                            output.write_all(b"[").await.unwrap();
                            open_list = true;
                        }

                        let msg = match format.as_str() {
                            "text" => proxy.as_text(),
                            "json" => proxy.as_json(),
                            _ => format!("{}", proxy),
                        };

                        output.write_all(msg.as_bytes()).await.unwrap();
                        if stop {
                            output
                                .write_all(if format == "json" { b"]" } else { b"" })
                                .await
                                .unwrap();
                        } else if format == "json" {
                            output.write_all(b",").await.unwrap();
                        }
                        output.write_all(b"\n").await.unwrap();
                    }
                    if limit != 0 {
                        counter -= 1;
                    }

                    if stop {
                        std::process::exit(0);
                    }
                }
            }
        });
}
