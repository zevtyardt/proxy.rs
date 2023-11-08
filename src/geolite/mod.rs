use crate::utils::get_data_dir;

pub mod downloader;
pub mod lookup;

#[derive(Debug)]
pub struct GeoData {
    pub iso_code: &'static str,
    pub name: &'static str,
    pub region_iso_code: &'static str,
    pub region_name: &'static str,
    pub city_name: &'static str,
}

impl GeoData {
    pub fn new(
        iso_code: &'static str,
        name: &'static str,
        region_iso_code: &'static str,
        region_name: &'static str,
        city_name: &'static str,
    ) -> Self {
        Self {
            iso_code,
            name,
            region_iso_code,
            region_name,
            city_name,
        }
    }
}

pub fn geolite_exists() -> bool {
    let geofile = get_data_dir(Some("geolite.db"));
    geofile.exists()
}
