use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;

pub const GEOLITEDB_DOWNLOAD_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/data/GeoLite2-City.mmdb";
pub const GEOLITEDB_CHECKSUM_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/data/Geolite2-City.mmdb.checksum";

pub fn download_geolite_file() -> anyhow::Result<()> {
    let bar = ProgressBar::new(100000000);
    bar.set_style(ProgressStyle::with_template(
        format!(
            "{}  Downloading GeoLite2-City.mmdb {} {{percent}}% {{bytes}}/{{total_bytes}} ({{bytes_per_sec}}, {{eta}})",
            "INFO".bright_green(),
            "~".fg_rgb::<128, 128, 128>()
        )
        .as_str(),
    )?);
    bar.finish();
    log::info!("done");
    Ok(())
}
