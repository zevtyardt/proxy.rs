use std::fs::remove_dir_all;

use anyhow::Context;

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
        remove_dir_all(geofile.as_path()).context(error_context!())?;
    }
    if !geofile.is_file() {
        return Ok(false);
    }

    // twst
    let my_ip = my_ip().await.context(error_context!())?;

    let mut averages = vec![];
    for i in 0..1000 {
        let instant = tokio::time::Instant::now();
        let result = geo_lookup(my_ip).await.context(error_context!())?;
        let e = instant.elapsed();
        averages.push(e.clone());
        log::info!("elapsed-{}: {:#?}", i + 1, e);
        if i == 0 {
            log::info!("{:#?}", result);
        }
    }
    let total = averages.len();
    let sum: tokio::time::Duration = averages.iter().sum();
    let average = sum / total as u32;
    log::info!("average: {:#?}", average);

    Ok(true)
}
