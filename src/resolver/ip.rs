use std::net::IpAddr;

use anyhow::Context;
use hyper::{Body, Request};
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{error_context, utils::hyper_client};

lazy_static! {
    static ref IP: Mutex<IpAddr> = Mutex::new(
        "127.0.0.1"
            .parse::<IpAddr>()
            .context(error_context!())
            .unwrap()
    );
}

pub async fn my_ip() -> anyhow::Result<IpAddr> {
    let mut local_ip = IP.lock().await;
    if !local_ip.to_string().eq("127.0.0.1") {
        return Ok(*local_ip);
    }

    let public_ip_source = vec![
        "https://wtfismyip.com/text",
        "http://api.ipify.org/",
        "http://ipinfo.io/ip",
        "http://ipv4.icanhazip.com/",
        "http://myexternalip.com/raw",
        "http://ipinfo.io/ip",
        "http://ifconfig.io/ip",
    ];

    let client = hyper_client();
    for source in public_ip_source {
        let request = Request::builder()
            .uri(source)
            .body(Body::empty())
            .context(error_context!())?;
        let response = client.request(request).await.context(error_context!())?;
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .context(error_context!())?;
        let body_str = String::from_utf8_lossy(&body);
        match body_str.trim().parse::<IpAddr>() {
            Ok(ip) => {
                log::debug!("ext ip ({}) retrieved using host: {}", ip, source);
                *local_ip = ip;
                return Ok(*local_ip);
            }
            Err(_) => continue,
        }
    }
    anyhow::bail!("unable to retrieve ip address for this machine")
}
