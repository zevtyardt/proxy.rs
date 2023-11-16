#![allow(dead_code)]
#![allow(unused_variables)]

use anyhow::Context;

use crate::{
    geolite::{check_geolite_db, downloader::download_geolite_db},
    utils::{logger::setup_logger, tokio_runtime},
};

mod checkers;
mod geolite;
mod proxy;
mod resolver;
mod utils;

fn start_app() -> anyhow::Result<()> {
    tokio_runtime()?.block_on(async {
        setup_logger(Some(log::LevelFilter::Debug)).context(error_context!())?;
        if !check_geolite_db().await.context(error_context!())? {
            download_geolite_db().await.context(error_context!())?;
            return Ok(());
        }
        Ok(())
    })
}

fn main() {
    let instant = tokio::time::Instant::now();
    if let Err(err) = start_app() {
        log::error!("{:?}", err)
    }
    log::info!("end: {:#?}", instant.elapsed())
}
