#![allow(dead_code)]
#![allow(unused)]

use std::process::exit;

use owo_colors::OwoColorize;

use crate::geolite::{geolite_exists, lookup::geo_lookup};

mod geolite;
mod proxies;
mod utils;

fn setup_logging() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} {}",
                record.level().green(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Off)
        .level_for("proxy_rs", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
}

fn main() {
    setup_logging();
    if geolite_exists() {
        log::info!("db tidak ada");
        exit(0);
    }

    let proxy = geo_lookup("114.142.169.20");
    log::info!("{:#?}", proxy);
}
