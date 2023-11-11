#![allow(dead_code)]
#![allow(unused_variables)]

use anyhow::Context;

use crate::{
    geolite::{check_geolite_db, downloader::download_geolite_db},
    utils::{logger::setup_logger, tokio_runtime},
};

mod geolite;
mod proxies;
mod resolver;
mod utils;

fn start_app() -> anyhow::Result<()> {
    tokio_runtime()?.block_on(async {
        setup_logger(Some(log::LevelFilter::Debug)).context(error_context!())?;
        if !check_geolite_db().await.context(error_context!())? {
            download_geolite_db().await.context(error_context!())?;
            std::process::exit(0);
        }

        Ok(())
    })
}

fn main() {
    if let Err(err) = start_app() {
        log::error!("{:?}", err)
    }
}
