use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    net::IpAddr,
    pin::Pin,
    str::from_utf8,
    task::{Context, Poll},
    time::Duration,
};

use native_tls::TlsConnector;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::TcpStream,
    time::{timeout, Instant},
};
use tokio_native_tls::TlsStream;

use crate::{
    resolver::{GeoData, Resolver},
    utils::{
        http::response::ResponseParser,
        serializer::{Country, Geo, ProxyData, ProxyType, Region},
    },
};

fn bytes_to_string(bytes: &[u8]) -> String {
    match from_utf8(bytes) {
        Ok(s) => format!("{:?}", s),
        Err(_) => {
            let v: Vec<String> = bytes.iter().map(|n| format!("{:02x}", n)).collect();
            format!("\"\\x{}\"", v.join("\\x"))
        }
    }
}

#[derive(Debug)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub expected_types: Vec<String>,
    pub geo: GeoData,
    pub types: Vec<(String, Option<String>)>,
    pub schemes: Vec<String>,
    pub logs: Vec<(String, String, Duration)>,
    pub negotiator_proto: String,

    pub verify_ssl: bool,
    pub timeout: i32,
    pub runtimes: Vec<f64>,

    pub tcp_stream: Option<TcpStream>,
    pub tls_stream: Option<TlsStream<TcpStream>>,

    pub request_stat: i32,
    pub error_stat: BTreeMap<String, i32>,

    pub is_working: bool,
}

impl Proxy {
    pub async fn create(host: &str, port: u16, expected_types: Vec<String>) -> Option<Self> {
        let mut host = host.to_string();
        let resolver = Resolver::new();
        if !resolver.host_is_ip(&host) {
            host = resolver.resolve(host).await;
        }
        if let Ok(ip_address) = host.parse::<IpAddr>() {
            let geo = resolver.get_ip_info(ip_address).await;

            return Some(Proxy {
                host,
                port,
                expected_types,
                geo,
                types: vec![],
                schemes: vec![],
                logs: vec![],
                negotiator_proto: "HTTP".to_string(),
                timeout: 5,
                runtimes: vec![],
                tcp_stream: None,
                tls_stream: None,
                verify_ssl: false,
                request_stat: 0,
                error_stat: BTreeMap::new(),
                is_working: false,
            });
        }
        None
    }

    pub fn error_rate(&self) -> f64 {
        if self.request_stat == 0 {
            return 0.0;
        }
        let sum = self.error_stat.values().sum::<i32>() as f64;
        sum / self.request_stat as f64
    }

    pub fn avg_resp_time(&self) -> f64 {
        if self.runtimes.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.runtimes.iter().sum();
        sum / self.runtimes.len() as f64
    }

    pub fn priority(&self) -> (f64, f64) {
        (self.error_rate(), self.avg_resp_time())
    }

    pub fn get_schemes(&mut self) -> Vec<String> {
        if self.schemes.is_empty() {
            for (proxy_type, _) in &self.types {
                if !self.schemes.contains(&"HTTP".to_string())
                    && ["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"].contains(&proxy_type.as_str())
                {
                    self.schemes.push("HTTP".to_string());
                }
                if !self.schemes.contains(&"HTTPS".to_string())
                    && ["HTTPS", "SOCKS4", "SOCKS5"].contains(&proxy_type.as_str())
                {
                    self.schemes.push("HTTPS".to_string());
                }
            }
        }
        self.schemes.clone()
    }

    pub fn as_text(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn as_json(&self) -> String {
        let proxy_data = ProxyData {
            host: self.host.clone(),
            port: self.port,
            geo: Geo {
                country: Country {
                    code: self.geo.iso_code.clone(),
                    name: self.geo.name.clone(),
                },
                region: Region {
                    code: self.geo.region_iso_code.clone(),
                    name: self.geo.region_name.clone(),
                },
                city: self.geo.city_name.clone(),
            },
            types: self
                .types
                .clone()
                .into_iter()
                .map(|(proxy_type, level)| ProxyType { proxy_type, level })
                .collect(),
            avg_resp_time: self.avg_resp_time(),
            error_rate: self.error_rate(),
        };

        serde_json::to_string(&proxy_data).unwrap()
    }

    pub fn log(&mut self, msg: &str, stime: Option<Duration>, error: Option<String>) {
        let runtime = if let Some(stime) = stime {
            self.runtimes.push(stime.as_secs_f64());
            stime
        } else {
            Duration::from_micros(0)
        };
        log::debug!(
            "{}:{} [{}] {}, Runtime {:?}",
            self.host,
            self.port,
            self.negotiator_proto,
            msg,
            runtime
        );

        self.logs
            .push((self.negotiator_proto.clone(), msg.to_string(), runtime));

        if let Some(error) = error {
            if !self.error_stat.contains_key(&error) {
                self.error_stat.insert(error.clone(), 0);
            }
            if let Some(value) = self.error_stat.get_mut(&error) {
                *value += 1
            }
        }
    }

    pub async fn connect(&mut self) -> bool {
        self.tcp_stream = self.connect_tcp().await;
        self.tcp_stream.is_some()
    }

    pub async fn send(&mut self, body: &[u8]) -> bool {
        let stime = Instant::now();
        match self.write_all(body).await {
            Ok(_) => {
                self.log(
                    format!("Sending {} bytes: {}", body.len(), bytes_to_string(body)).as_str(),
                    Some(stime.elapsed()),
                    None,
                );
                true
            }
            Err(e) => {
                self.log(
                    format!("Sending error: {}", e).as_str(),
                    Some(stime.elapsed()),
                    Some(e.to_string()),
                );
                false
            }
        }
    }

    pub async fn recv(&mut self, size: usize) -> Option<Vec<u8>> {
        let stime = Instant::now();
        let mut chunk = vec![0; size];

        match timeout(
            Duration::from_secs(self.timeout as u64),
            self.read_exact(&mut chunk),
        )
        .await
        {
            Ok(buffer) => match buffer {
                Ok(buf_size) => {
                    if buf_size > 0 {
                        self.log(
                            format!(
                                "Received {} bytes: {}",
                                buf_size,
                                bytes_to_string(&chunk[..buf_size])
                            )
                            .as_str(),
                            Some(stime.elapsed()),
                            None,
                        );
                        return Some(chunk.to_vec());
                    }
                }
                Err(e) => self.log(
                    format!("Failed to receive: {}", e).as_str(),
                    Some(stime.elapsed()),
                    Some(e.to_string()),
                ),
            },
            Err(e) => self.log(
                format!("Received timeout: {}", e).as_str(),
                Some(stime.elapsed()),
                Some(e.to_string()),
            ),
        }
        None
    }

    pub async fn recv_all(&mut self) -> Option<Vec<u8>> {
        let stime = Instant::now();
        let mut chunk = vec![0; 1024];
        let mut buf = Vec::new();
        loop {
            match timeout(
                Duration::from_secs(self.timeout as u64),
                self.read(&mut chunk),
            )
            .await
            {
                Ok(buffer) => match buffer {
                    Ok(buf_size) => {
                        if buf_size == 0 {
                            break;
                        }
                        let data = &chunk[0..buf_size];
                        buf.extend(data)
                    }
                    Err(e) => {
                        self.log(
                            format!("Failed to receive: {}", e).as_str(),
                            Some(stime.elapsed()),
                            Some(e.to_string()),
                        );
                        break;
                    }
                },

                Err(e) => {
                    self.log(
                        format!("Received timeout: {}", e).as_str(),
                        Some(stime.elapsed()),
                        Some(e.to_string()),
                    );
                    // TODO: log error
                    break;
                }
            }
        }
        self.log(
            format!(
                "Received {} bytes: {}",
                buf.len(),
                bytes_to_string(buf.as_slice())
            )
            .as_str(),
            Some(stime.elapsed()),
            None,
        );
        Some(buf)
    }

    pub async fn close(&mut self) {
        if self.tcp_stream.is_some() {
            self.close_tls().await;
        }
        if self.tls_stream.is_some() {
            self.close_tcp().await;
        }
    }
}

impl Ord for Proxy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.error_rate()
            .partial_cmp(&other.error_rate())
            .unwrap()
            .reverse()
            .then(
                self.avg_resp_time()
                    .partial_cmp(&other.avg_resp_time())
                    .unwrap()
                    .reverse(),
            )
    }
}

impl PartialOrd for Proxy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for Proxy {}

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.port == other.port
    }
}

impl std::fmt::Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut types = vec![];
        for (k, v) in &self.types {
            if let Some(v) = v {
                types.push(format!("{}: {}", k, v));
            } else {
                types.push(k.to_string())
            }
        }

        write!(
            f,
            "<Proxy {} {:.2}s [{}] {}:{}>",
            self.geo.iso_code,
            self.avg_resp_time(),
            types.join(", "),
            self.host,
            self.port
        )
    }
}

impl AsyncRead for Proxy {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        if let Some(ref mut tcp_stream) = self.tcp_stream {
            Pin::new(tcp_stream).poll_read(cx, buf)
        } else if let Some(ref mut tls_stream) = self.tls_stream {
            Pin::new(tls_stream).poll_read(cx, buf)
        } else {
            Poll::Ready(Err(Error::new(ErrorKind::Other, "No Stream Available")))
        }
    }
}

impl AsyncWrite for Proxy {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        if let Some(ref mut tcp_stream) = self.tcp_stream {
            Pin::new(tcp_stream).poll_write(cx, buf)
        } else if let Some(ref mut tls_stream) = self.tls_stream {
            Pin::new(tls_stream).poll_write(cx, buf)
        } else {
            Poll::Ready(Err(Error::new(ErrorKind::Other, "No Stream Available")))
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if let Some(ref mut tcp_stream) = self.tcp_stream {
            Pin::new(tcp_stream).poll_flush(cx)
        } else if let Some(ref mut tls_stream) = self.tls_stream {
            Pin::new(tls_stream).poll_flush(cx)
        } else {
            Poll::Ready(Err(Error::new(ErrorKind::Other, "No Stream Available")))
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if let Some(ref mut tcp_stream) = self.tcp_stream {
            Pin::new(tcp_stream).poll_shutdown(cx)
        } else if let Some(ref mut tls_stream) = self.tls_stream {
            Pin::new(tls_stream).poll_shutdown(cx)
        } else {
            Poll::Ready(Err(Error::new(ErrorKind::Other, "No Stream Available")))
        }
    }
}

// TCP
impl Proxy {
    async fn connect_tcp(&mut self) -> Option<TcpStream> {
        let stime = Instant::now();
        self.log("Initial connection", Some(stime.elapsed()), None);
        match timeout(
            Duration::from_secs(self.timeout as u64),
            TcpStream::connect(self.as_text()),
        )
        .await
        {
            Ok(stream) => match stream {
                Ok(stream) => {
                    self.log("Connection success", Some(stime.elapsed()), None);
                    self.request_stat += 1;
                    Some(stream)
                }
                Err(e) => {
                    self.log(
                        format!("Connection error: {}", e).as_str(),
                        Some(stime.elapsed()),
                        Some(e.to_string()),
                    );
                    None
                }
            },
            Err(e) => {
                self.log("Connection timeout", None, Some(e.to_string()));
                None
            }
        }
    }

    async fn close_tcp(&mut self) {
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.shutdown().await {
                Ok(_) => self.log("Connection closed", None, None),
                Err(e) => self.log(
                    format!("Failed to close connection: {}", e).as_str(),
                    None,
                    Some(e.to_string()),
                ),
            }
        }
        self.tcp_stream = None;
    }
}

// TLS / SSL
impl Proxy {
    /// Only used to check the https protocol not for servers.
    pub async fn connect_ssl(&mut self, connect_payload: &[u8]) -> bool {
        let tcp_stream = self.connect_tcp().await;
        if tcp_stream.is_none() {
            return false;
        }
        let mut tcp_stream = tcp_stream.unwrap();

        // send connnect requests
        let stime_send = Instant::now();
        match tcp_stream.write_all(connect_payload).await {
            Ok(_) => self.log(
                format!(
                    "Sending {} bytes: {}",
                    connect_payload.len(),
                    bytes_to_string(connect_payload)
                )
                .as_str(),
                Some(stime_send.elapsed()),
                None,
            ),
            Err(e) => {
                self.log(
                    format!("Sending error: {}", e).as_str(),
                    Some(stime_send.elapsed()),
                    Some(e.to_string()),
                );
                return false;
            }
        };

        // recv response
        let stime_recv = Instant::now();
        let mut chunk = vec![0; 2048];
        match timeout(
            Duration::from_secs(self.timeout as u64),
            tcp_stream.read(&mut chunk),
        )
        .await
        {
            Ok(buffer) => match buffer {
                Ok(buf_size) => {
                    if buf_size > 0 {
                        self.log(
                            format!(
                                "Received {} bytes: {}",
                                buf_size,
                                bytes_to_string(&chunk[..buf_size])
                            )
                            .as_str(),
                            Some(stime_recv.elapsed()),
                            None,
                        );
                    }
                }
                Err(e) => {
                    self.log(
                        format!("Failed to receive: {}", e).as_str(),
                        Some(stime_recv.elapsed()),
                        Some(e.to_string()),
                    );
                    return false;
                }
            },
            Err(e) => {
                self.log(
                    format!("Received timeout: {}", e).as_str(),
                    Some(stime_recv.elapsed()),
                    Some(e.to_string()),
                );
                return false;
            }
        }

        let response = ResponseParser::parse(&chunk);
        if response.status_code.unwrap_or(0) != 200 {
            return false;
        }

        let stime = Instant::now();
        self.log("SSL: Initial connection", Some(stime.elapsed()), None);

        let config = TlsConnector::builder()
            .danger_accept_invalid_certs(!self.verify_ssl)
            .build()
            .unwrap();
        let connector = tokio_native_tls::TlsConnector::from(config);
        self.tls_stream = match timeout(
            Duration::from_secs(self.timeout as u64),
            connector.connect(&self.host, tcp_stream),
        )
        .await
        {
            Ok(stream) => match stream {
                Ok(stream) => {
                    self.log("SSL: Connection success", Some(stime.elapsed()), None);
                    self.request_stat += 1;
                    Some(stream)
                }
                Err(e) => {
                    self.log(
                        format!("SSL: Connection error: {}", e).as_str(),
                        Some(stime.elapsed()),
                        Some(e.to_string()),
                    );
                    None
                }
            },
            Err(e) => {
                self.log(
                    "SSL: Connection timeout",
                    Some(stime.elapsed()),
                    Some(e.to_string()),
                );
                None
            }
        };

        self.tls_stream.is_some()
    }

    async fn close_tls(&mut self) {
        if let Some(stream) = self.tls_stream.as_mut() {
            match stream.shutdown().await {
                Ok(_) => self.log("SSL: Connection closed", None, None),
                Err(e) => self.log(
                    format!("SSL: Failed to close connection: {}", e).as_str(),
                    None,
                    Some(e.to_string()),
                ),
            }
        }
        self.tcp_stream = None;
    }
}
