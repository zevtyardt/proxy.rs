use anyhow::Context;
use tokio::fs::remove_dir;

use crate::{error_context, geolite::lookup::geo_lookup, resolver::ip::my_ip, utils::get_data_dir};

pub mod downloader;
pub mod lookup;

pub const GEOLITEDB: &str = "GeoLite2-City.mmdb";

#[derive(Debug)]
pub struct GeoData {
    pub iso_code: String,
    pub name: String,
    pub region_iso_code: String,
    pub region_name: String,
    pub city_name: String,
}

impl Default for GeoData {
    fn default() -> Self {
        Self {
            iso_code: String::from("--"),
            name: String::from("Unknown"),
            region_iso_code: String::from("Unknown"),
            region_name: String::from("Unknown"),
            city_name: String::from("Unknown"),
        }
    }
}

pub async fn check_geolite_db() -> anyhow::Result<bool> {
    let geofile = get_data_dir(Some(GEOLITEDB))
        .await
        .context(error_context!())?;

    if geofile.is_dir() {
        remove_dir(geofile.as_path())
            .await
            .context(error_context!())?;
    }
    if !geofile.is_file() {
        return Ok(false);
    }

    let ip = my_ip().await.context(error_context!())?;
    if let Err(err) = geo_lookup(ip).await {
        log::error!("{:?}", err);
        return Ok(false);
    }

    Ok(true)
}
