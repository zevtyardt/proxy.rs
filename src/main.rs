use lazy_static::lazy_static;
use std::net::IpAddr;

use crate::{judge::get_judges, resolver::Resolver};
mod judge;
mod resolver;
mod utils;

lazy_static! {
    pub static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
}
fn main() {
    std::env::set_var("RUST_LOG", "rproxy=debug");
    pretty_env_logger::init();

    RUNTIME.block_on(async {
        let resolver = Resolver::new();
        let ext_ip = resolver.get_real_ext_ip().await.unwrap();
        let my_ip: IpAddr = ext_ip.parse().unwrap();
        resolver.get_ip_info(my_ip).await;

        let c_resolver = resolver.clone();
        tokio::task::spawn_blocking(move || {
            c_resolver.resolve("yahoo.com".to_string());
            c_resolver.resolve("google.com".to_string());
        })
        .await
        .unwrap();

        get_judges().await;
    })
}

#[cfg(test)]
mod test;
