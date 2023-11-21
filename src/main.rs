#![allow(dead_code)]
#![allow(unused_variables)]

use std::sync::Arc;

use anyhow::Context;
use futures_util::{stream::FuturesUnordered, StreamExt};
use judges::{
    http::{get_http_judge, init_http_judge},
    https::{get_https_judge, init_https_judge},
    smtp::get_smtp_judge,
};
use tokio::sync::Semaphore;

use crate::{
    geolite::{check_geolite_db, downloader::download_geolite_db},
    utils::{logger::setup_logger, tokio_runtime},
};

mod checkers;
mod geolite;
mod judges;
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

        tokio::spawn(async {
            log::info!("http init");
            (
                "http",
                init_http_judge()
                    .await
                    .context(error_context!())
                    .unwrap_or(false),
            )
        });
        tokio::spawn(async {
            log::info!("https init");
            (
                "https",
                init_https_judge()
                    .await
                    .context(error_context!())
                    .unwrap_or(false),
            )
        });

        log::info!("main");
        let mut fut = FuturesUnordered::new();
        let sem = Arc::new(Semaphore::new(500));

        for i in 0..10 {
            let permit = Arc::clone(&sem).acquire_owned().await;
            fut.push(async move {
                let _ = permit;
                if let Ok(url) = get_smtp_judge().await.context(error_context!()) {
                    log::info!("{:?}", url);
                }
                if let Ok(url) = get_http_judge().await.context(error_context!()) {
                    log::info!("{:?}", url);
                }
                if let Ok(url) = get_https_judge().await.context(error_context!()) {
                    log::info!("{:?}", url);
                }
            })
        }
        while (fut.next().await).is_some() {
            continue;
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
