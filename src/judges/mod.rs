use std::{fmt::Display, sync::Arc, time::Duration};

use anyhow::Context;
use futures_util::{stream::FuturesUnordered, StreamExt};
use hyper::{client::HttpConnector, header::USER_AGENT, Body, Client, Request, StatusCode};
use hyper_tls::HttpsConnector;
use tokio::{sync::Semaphore, time::timeout};
use ua_generator::ua::spoof_ua;
use url::Url;

use crate::{error_context, resolver::ip::my_ip};

pub mod http;
pub mod https;
pub mod smtp;

#[derive(Debug, Clone)]
pub struct Judge {
    pub scheme: String,
    pub host: String,
    pub path: String,
}

impl Display for Judge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}{}", self.scheme, self.host, self.path)
    }
}

pub fn parse_url(url: &str) -> anyhow::Result<Judge> {
    let parsed = Url::parse(url).context(error_context!())?;
    if !parsed.has_host() {
        anyhow::bail!("url does not have a valid host");
    }
    Ok(Judge {
        scheme: parsed.scheme().to_uppercase(),
        host: parsed.host_str().unwrap().to_string(),
        path: parsed.path().to_string(),
    })
}

async fn check_all_host(hosts: Vec<&str>) -> anyhow::Result<Vec<Judge>> {
    let mut results = vec![];
    let mut fut = FuturesUnordered::new();
    let sem = Arc::new(Semaphore::new(5));

    for host in hosts {
        let permit = Arc::clone(&sem).acquire_owned().await;
        let host = parse_url(host).context(error_context!())?;
        fut.push(async move {
            let _ = permit;
            match check_judge_host(&host).await.context(error_context!()) {
                Ok(is_working) => {
                    if is_working {
                        return Some(host);
                    }
                }
                Err(err) => log::debug!("Error: {:?}", err),
            }
            None
        })
    }
    while let Some(result) = fut.next().await {
        if let Some(host) = result {
            results.push(host);
        }
    }
    Ok(results)
}

async fn check_judge_host(judge: &Judge) -> anyhow::Result<bool> {
    let connector = hyper_tls::native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map(|tls| {
            let mut http = HttpConnector::new();
            http.enforce_http(false);
            HttpsConnector::from((http, tls.into()))
        })
        .context(error_context!())?;
    let client = Client::builder().build::<_, Body>(connector);

    let ua = spoof_ua();
    let request = Request::builder()
        .uri(judge.to_string())
        .header(USER_AGENT, ua)
        .body(Body::empty())
        .context(error_context!())?;

    let response = timeout(Duration::from_secs(3), client.request(request))
        .await
        .context(error_context!())?
        .context(error_context!())?;
    if response.status() == StatusCode::OK {
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .context(error_context!())?;
        let body_str = String::from_utf8_lossy(&body);
        let ip = my_ip().await.context(error_context!())?;
        if body_str.contains(&ip.to_string()) {
            return Ok(true);
        }
    }

    Ok(false)
}
