use super::GeoData;

pub fn geo_lookup(ip: &str) -> GeoData {
    GeoData::new("--", "Unknown", "Unknown", "Unknown", "Unknown")
}
