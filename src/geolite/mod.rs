use std::fs::remove_dir_all;

use anyhow::Context;

use crate::{error_context, geolite::lookup::geo_lookup, resolver::ip::MY_IP, utils::get_data_dir};

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
        remove_dir_all(geofile.as_path()).context(error_context!())?;
    }
    if !geofile.is_file() {
        return Ok(false);
    }

    let my_ip = MY_IP.get().await;
    if let Err(err) = geo_lookup(*my_ip).await {
        log::error!("{:?}", err);
        return Ok(false);
    }

    Ok(true)
}
