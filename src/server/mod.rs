pub mod proxy_pool;

use std::time::Duration;

use hyper::{server::conn::Http, service::service_fn, Body, Method, Request, Response, StatusCode};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

use self::proxy_pool::{ProxyPool, SimpleProxy, LIVE_PROXIES};
use crate::utils::http::response::ResponseParser;

lazy_static! {
    static ref POOL: Mutex<ProxyPool> = Mutex::new(ProxyPool::new());
}

const TIMEOUT_IN_SECONDS: u64 = 8;

#[derive(Debug)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

impl Server {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }

    pub async fn start(&self) {
        while LIVE_PROXIES.is_empty() {
            continue;
        }

        let addr = format!("{}:{}", self.host, self.port);
        if let Ok(listener) = TcpListener::bind(&addr).await {
            log::info!("Listening on http://{}", addr);

            loop {
                if let Ok((stream, addr)) = listener.accept().await {
                    log::info!("Accepted connection from {}", addr);
                    tokio::task::spawn(async move {
                        if let Err(err) = Http::new()
                            .http1_title_case_headers(true)
                            .http1_title_case_headers(true)
                            .serve_connection(stream, service_fn(handle_stream))
                            .with_upgrades()
                            .await
                        {
                            log::error!("Connection error: {}", err);
                        }
                    });
                }
            }
        }
    }
}

async fn handle_stream(request: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    if let Some(mut proxy) = get_proxy(request.method()) {
        log::info!("Proxying to: {}", proxy.as_text());

        if request.method() == Method::CONNECT {
            tokio::task::spawn(async move {
                if let Err(err) = handle_connect_stream(request, proxy).await {
                    log::error!("Failed to connect proxy: {}", err);
                }
            });
            Ok(Response::new(Body::empty()))
        } else {
            let proxy_stream = TcpStream::connect(proxy.as_text()).await.unwrap();
            if let Ok((mut sender, conn)) = hyper::client::conn::Builder::new()
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .handshake(proxy_stream)
                .await
            {
                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        log::error!("Failed to connect proxy: {}", err);
                    }
                });
                let response = sender.send_request(request).await;
                proxy.request_stat += 1;
                POOL.lock().put(proxy);
                response
            } else {
                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::empty())
                    .unwrap())
            }
        }
    } else {
        Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::empty())
            .unwrap())
    }
}

async fn handle_connect_stream(
    request: Request<Body>,
    mut proxy: SimpleProxy,
) -> Result<(), Box<dyn std::error::Error>> {
    let uri = request.uri().clone();
    if let Some(host) = uri.host() {
        if let Ok(mut upgrade) = hyper::upgrade::on(request).await {
            let mut proxy_stream = TcpStream::connect(proxy.as_text()).await?;
            let connect_status =
                send_connect_request(&mut proxy_stream, host, TIMEOUT_IN_SECONDS).await;

            if connect_status {
                tokio::io::copy_bidirectional(&mut upgrade, &mut proxy_stream).await?;
                proxy.request_stat += 1;
                POOL.lock().put(proxy);
            }
        }
    }
    Ok(())
}

fn get_proxy(method: &Method) -> Option<SimpleProxy> {
    let mut pool = POOL.lock();
    if method == Method::CONNECT {
        pool.get("HTTPS")
    } else {
        pool.get("HTTP")
    }
}

async fn send_connect_request<R: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut R,
    host: &str,
    timeout_in_seconds: u64,
) -> bool {
    let connect = format!(
        "CONNECT {0}:443 HTTP/1.1\r\nHost: {0}:443\r\nProxy-Connection: Keep-Alive\r\n\r\n",
        host
    );
    // Send data
    if let Ok(Ok(_)) = timeout(
        Duration::from_secs(timeout_in_seconds),
        stream.write_all(connect.as_bytes()),
    )
    .await
    {
        // read Response
        let data = read_timeout(stream, timeout_in_seconds).await;
        let response = ResponseParser::parse(data.as_slice());

        if let Some(status_code) = response.status_code {
            return status_code == 200;
        }
    }
    false
}

async fn read_timeout<R: AsyncRead + Unpin>(reader: &mut R, timeout_in_seconds: u64) -> Vec<u8> {
    let mut data = vec![];
    loop {
        let mut buf = [0; 512];
        if let Ok(Ok(buf_size)) = timeout(
            Duration::from_secs(timeout_in_seconds),
            reader.read(&mut buf),
        )
        .await
        {
            if buf_size == 0 {
                break;
            }
            data.extend(&buf[..buf_size]);
            continue;
        }
        break;
    }
    data
}
