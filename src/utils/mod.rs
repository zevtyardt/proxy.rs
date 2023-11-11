use anyhow::Context;
use dirs::{data_dir, data_local_dir};
use hyper::{client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;
use std::path::PathBuf;
use tokio::{
    fs::create_dir_all,
    runtime::{Builder, Runtime},
};

use crate::error_context;

pub mod error;
pub mod logger;

pub async fn get_data_dir(file: Option<&str>) -> anyhow::Result<PathBuf> {
    let mut path = if let Some(path) = data_dir() {
        path
    } else if let Some(path) = data_local_dir() {
        path
    } else {
        PathBuf::from("./")
    };
    path.push("proxy-rs/");
    if !path.is_dir() {
        create_dir_all(&path).await.context(error_context!())?;
    }
    if let Some(file) = file {
        path.push(file);
    }
    Ok(path)
}

pub fn tokio_runtime() -> anyhow::Result<Runtime> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .context(error_context!())?;
    Ok(runtime)
}

pub fn hyper_client() -> Client<HttpsConnector<HttpConnector>> {
    let https_connector = HttpsConnector::new();
    Client::builder().build::<_, Body>(https_connector)
}
