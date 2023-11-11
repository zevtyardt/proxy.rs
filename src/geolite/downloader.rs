use anyhow::Context;
use hyper::{body::HttpBody, header::CONTENT_LENGTH, Body, Request};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    error_context,
    utils::{get_data_dir, hyper_client},
};

use super::GEOLITEDB;

pub const GEOLITEDB_DOWNLOAD_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/data/GeoLite2-City.mmdb";
pub const GEOLITEDB_CHECKSUM_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/data/Geolite2-City.mmdb.checksum";

pub async fn download_geolite_db() -> anyhow::Result<()> {
    let bar = ProgressBar::new(0);
    bar.set_style(ProgressStyle::with_template(
        format!(
            "{}  Downloading GeoLite2-City.mmdb {} {{percent}}% {{bytes}}/{{total_bytes}} ({{bytes_per_sec}}, {{eta}})",
            "INFO".bright_green(),
            "~".fg_rgb::<128, 128, 128>()
        )
        .as_str(),
    )?);

    let client = hyper_client();
    let request = Request::builder()
        .uri(GEOLITEDB_DOWNLOAD_URL)
        .body(Body::empty())
        .context(error_context!())?;

    let path = get_data_dir(Some(GEOLITEDB))
        .await
        .context(error_context!())?;

    let mut file = File::create(&path).await.context(error_context!())?;
    let response = client.request(request).await.context(error_context!())?;

    let headers = response.headers();
    if let Some(content_length) = headers.get(CONTENT_LENGTH) {
        let content_length = content_length.to_str().context(error_context!())?;
        bar.set_length(content_length.parse::<u64>().context(error_context!())?);
    }

    let mut body = response.into_body();
    while let Some(Ok(bytes)) = body.data().await {
        if file.write_all(&bytes).await.is_ok() {
            bar.inc(bytes.len() as u64);
        }
    }
    bar.finish();

    Ok(())
}
