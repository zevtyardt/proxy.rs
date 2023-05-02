use std::net::IpAddr;

use crate::{judge::Judge, resolver::Resolver};
mod judge;
mod resolver;
mod utils;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let resolver = Resolver::new();
            let ip = resolver.get_real_ext_ip().await.unwrap();

            let my_ip: IpAddr = ip.parse().unwrap();
            let lookup = resolver.get_ip_info(my_ip).await;
            println!("{lookup:#?}");

            for _ in 0..10 {
                let mut c_resolver = resolver.clone();
                tokio::task::spawn_blocking(move || {
                    let ip = c_resolver.resolve("yahoo.com".to_string());
                    println!("yahoo: {ip:#?}");
                    let ip = c_resolver.resolve("google.com".to_string());
                    println!("google: {ip:#?}")
                })
                .await
                .unwrap();
            }

            let judge = Judge::new("http://httpbin.org/get?show_env");
            println!("judge: {:#?}", judge.url);
        })
}
