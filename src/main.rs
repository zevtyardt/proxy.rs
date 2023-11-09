#![allow(dead_code)]
#![allow(unused)]

use std::process::exit;

use crate::{
    geolite::{downloader::download_geolite_file, geolite_exists, lookup::geo_lookup},
    utils::logger::setup_logger,
};

mod geolite;
mod proxies;
mod utils;

fn main() -> anyhow::Result<()> {
    setup_logger(Some(log::LevelFilter::Debug));

    if !geolite_exists() {
        download_geolite_file()?;
    }
    let proxy = geo_lookup("114.142.169.20");
    log::info!("{:#?}", proxy);

    Ok(())
}
