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
        let mut proxy =
            proxy::Proxy::create("188.166.218.206", 8080, utils::vec_of_strings!["HTTP"]).await;
        proxy.connect().await;
        proxy.send(b"GET http://azenv.net/ HTTP/1.1\r\nUser-Agent: PxBroker/0.4.0/9500\r\nAccept: */*\r\nAccept-Encoding: gzip, deflate\r\nPragma: no-cache\r\nCache-control: no-cache\r\nCookie: cookie=ok\r\nReferer: https://www.google.com/\r\nHost: azenv.net\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").await;
        if let Some(msg) = proxy.recv().await {
        println!("{}", msg)
        }

        /*
            futures_util::future::join_all(providers::get_all_tasks()).await;
            log::info!("Total proxies scraped: {}", providers::PROXIES.qsize(),);

            let mut dupe = vec![];
            let data = providers::PROXIES.data.lock().unwrap();
            for prox in data.clone().into_iter() {
                if dupe.contains(&prox) {
                    log::info!("Duplicate {}", prox);
                }
                dupe.push(prox)
            }

            let mut checker = checker::Checker::default();
            checker.verify_ssl = true;
            checker.check_judges().await
        */
    })
}
