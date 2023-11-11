use std::net::IpAddr;

use anyhow::Context;
use maxminddb::{geoip2::City, Reader};

use crate::{error_context, utils::get_data_dir};

use super::{GeoData, GEOLITEDB};

pub async fn geo_lookup(ip: IpAddr) -> anyhow::Result<GeoData> {
    let geofile = get_data_dir(Some(GEOLITEDB))
        .await
        .context(error_context!())?;

    let mut geodata = GeoData::default();
    let db = Reader::open_readfile(geofile).context(error_context!())?;
    let lookup = db.lookup::<City>(ip).context(error_context!())?;

    if let Some(country) = &lookup.country {
        if let Some(country_iso_code) = &country.iso_code {
            geodata.iso_code = country_iso_code.to_string()
        }
        if let Some(country_names) = &country.names {
            if let Some(country_name) = country_names.get("en") {
                geodata.name = country_name.to_string();
            }
        }
    } else if let Some(continent) = &lookup.continent {
        if let Some(continent_iso_code) = &continent.code {
            geodata.iso_code = continent_iso_code.to_string()
        }
        if let Some(continent_names) = &continent.names {
            if let Some(continent_name) = continent_names.get("en") {
                geodata.name = continent_name.to_string();
            }
        }
    }

    if let Some(subdivisions) = &lookup.subdivisions {
        if let Some(division) = subdivisions.first() {
            if let Some(division_iso_code) = &division.iso_code {
                geodata.region_iso_code = division_iso_code.to_string()
            }
            if let Some(division_names) = &division.names {
                if let Some(division_name) = division_names.get("en") {
                    geodata.region_name = division_name.to_string()
                }
            }
        }
    }

    if let Some(city) = &lookup.city {
        if let Some(city_names) = &city.names {
            if let Some(city_name) = city_names.get("en") {
                geodata.city_name = city_name.to_string()
            }
        }
    }

    Ok(geodata)
}
