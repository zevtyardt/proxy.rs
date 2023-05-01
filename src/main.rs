use std::net::IpAddr;

use crate::{providers::freeproxylist::FreeProxyListNetProvider, resolver::Resolver};
mod providers;
mod resolver;

fn main() {
    let mut resolver = Resolver::new();
    let ip = resolver.get_real_ext_ip().unwrap();
    let my_ip: Option<IpAddr> = match ip.parse() {
        Ok(ip) => Some(ip),
        Err(e) => {
            println!("{}", e);
            None
        }
    };

    if let Some(my_ip) = my_ip {
        let lookup = resolver.get_ip_info(my_ip);
        println!("{lookup:#?}");

        let ip = resolver.resolve("google.com");
        println!("real_ip: {ip:#?}")
    }

    let mut p = FreeProxyListNetProvider::new();
    p.add("tes");
    println!("{:#?}", p);
    println!("{}", p.url)
}
