const GITHUB_CARGO_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/Cargo.toml";

pub async fn check_version() {
    if let Ok(response) = reqwest::get(GITHUB_CARGO_URL).await {
        if let Ok(text) = response.text().await {
            let re = regex::Regex::new(r#"version = "([\d.]+)""#).unwrap();
            if let Some(cap) = re.captures(text.as_str()) {
                let latest_version = cap.get(1).unwrap().as_str();
                let current_version = env!("CARGO_PKG_VERSION");

                if latest_version != current_version {
                    log::warn!("Version Mismatch:\nLatest version detected: v{}\nCurrent version: v{}\n\nPlease update or reinstall for compatibility. For more information:\nvisit https://github.com/zevtyardt/proxy.rs\n", latest_version, current_version);
                }
            }
        }
    }
}
