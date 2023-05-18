#![allow(dead_code)]
#![allow(unused_variables)]
//#![allow(unused_imports)]

use lazy_static::lazy_static;

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
    std::env::set_var("RUST_LOG", "proxy_rs=debug");
    pretty_env_logger::init();

    RUNTIME.block_on(async {
        futures_util::future::join_all(providers::get_all_tasks()).await;
        log::info!("Total proxies scraped: {}", providers::PROXIES.qsize());

        let mut tasks: Vec<tokio::task::JoinHandle<()>> = vec![];
        while let Some(mut prox) = providers::PROXIES.get_nowait() {
            if tasks.len() == 2 {
                futures_util::future::join_all(&mut tasks).await;
                tasks.clear();
            }

            tasks.push(tokio::task::spawn(async move {
                prox.connect().await;

                if prox.stream.is_some() {
        prox.send(b"GET http://azenv.net/ HTTP/1.1\r\nUser-Agent: PxBroker/0.4.0/9500\r\nAccept: */*\r\nAccept-Encoding: gzip, deflate\r\nPragma: no-cache\r\nCache-control: no-cache\r\nCookie: cookie=ok\r\nReferer: https://www.google.com/\r\nHost: azenv.net\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").await;
                   if let Some(data) = prox.recv_all().await {
                        println!("{:?}", prox.logs);

                        let response = utils::http::Response::parse(data.as_slice());
                        println!("{:#?}, {}", response, prox);

                        /*if response.status_code.is_some() && response.status_code.unwrap() == 200 {
                            break;
                        }*/
                    }
                }
            }))
        }

        if !tasks.is_empty() {
            futures_util::future::join_all(&mut tasks).await;
            tasks.clear();
        }
    })
}

fn mauin() {
    std::env::set_var("RUST_LOG", "proxy_rs=debug");
    pretty_env_logger::init();

    RUNTIME.block_on(async {
        let mut proxy =
            proxy::Proxy::create("121.22.53.166", 9091, utils::vec_of_strings!["HTTP"]).await;
        proxy.connect().await;
        proxy.send(b"GET http://azenv.net/ HTTP/1.1\r\nUser-Agent: PxBroker/0.4.0/9500\r\nAccept: */*\r\nAccept-Encoding: gzip, deflate\r\nPragma: no-cache\r\nCache-control: no-cache\r\nCookie: cookie=ok\r\nReferer: https://www.google.com/\r\nHost: azenv.net\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").await;
        if let Some(data) = proxy.recv_all().await {
            let response = utils::http::Response::parse(data.as_slice());
            println!("{:?}", response);
        }
        println!("{}", proxy);
   })
}
