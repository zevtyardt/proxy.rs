use super::http::hyper_client;
use hyper::{Body, Request};

const GITHUB_CARGO_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/Cargo.toml";

pub async fn check_version() {
    let client = hyper_client();

    let request = Request::builder()
        .uri(GITHUB_CARGO_URL)
        .header("Cache-Control", "no-cache")
        .body(Body::empty())
        .unwrap();

    if let Ok(response) = client.request(request).await {
        if let Ok(body) = hyper::body::to_bytes(response.into_body()).await {
            let body_str = String::from_utf8_lossy(&body);
            if let Some(version) = body_str.lines().find(|p| p.starts_with("version")) {
                let latest_version = version
                    .trim_start_matches("version = \"")
                    .trim_end_matches("\"");
                let current_version = env!("CARGO_PKG_VERSION");

                if latest_version != current_version {
                    log::warn!(
                        "Version Mismatch:
Latest version detected: v{}
Current version: v{}

Please update by running the following command:
cargo install proxy-rs

For more information, please visit:
https://github.com/zevtyardt/proxy.rs
",
                        latest_version,
                        current_version
                    );
                }
            }
        }
    }
}
