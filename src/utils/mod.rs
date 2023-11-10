use anyhow::Context;
use dirs::{data_dir, data_local_dir};
use hyper::{client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;
use std::path::PathBuf;
use tokio::runtime::{Builder, Runtime};

use crate::debug_error;

pub mod error;
pub mod logger;

pub fn get_data_dir(file: Option<&str>) -> PathBuf {
    let mut path = if let Some(path) = data_dir() {
        path
    } else if let Some(path) = data_local_dir() {
        path
    } else {
        PathBuf::from("./")
    };
    path.push("proxy-rs/");
    if let Some(file) = file {
        path.push(file);
    }
    path
}

pub fn tokio_runtime() -> anyhow::Result<Runtime> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .context(debug_error!())?;
    Ok(runtime)
}

pub fn hyper_client() -> Client<HttpsConnector<HttpConnector>> {
    let https_connector = HttpsConnector::new();
    Client::builder().build::<_, Body>(https_connector)
}
