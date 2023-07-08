use hyper::{Body, Request};

use super::http::hyper_client;

const GITHUB_CARGO_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/Cargo.toml";

pub async fn check_version() {
    let client = hyper_client();

    let request = Request::builder()
        .uri(GITHUB_CARGO_URL)
        .body(Body::empty())
        .unwrap();

    if let Ok(response) = client.request(request).await {
        if let Ok(body) = hyper::body::to_bytes(response.into_body()).await {
            let body_str = String::from_utf8_lossy(&body);
            let re = regex::Regex::new(r#"version = "([\d.]+)""#).unwrap();
            if let Some(cap) = re.captures(&body_str) {
                let latest_version = cap.get(1).unwrap().as_str();
                let current_version = env!("CARGO_PKG_VERSION");

                if latest_version != current_version {
                    log::warn!("Version Mismatch:\nLatest version detected: v{}\nCurrent version: v{}\n\nPlease update or reinstall for compatibility. For more information:\nvisit https://github.com/zevtyardt/proxy.rs\n", latest_version, current_version);
                }
            }
        }
    }
}
