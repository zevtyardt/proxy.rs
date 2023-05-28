use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Duration};

use native_tls::TlsConnector;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Instant},
};
use tokio_native_tls::TlsStream;

use crate::{
    resolver::{GeoData, Resolver},
    utils::http::Response,
};

#[derive(Debug)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub expected_types: Vec<String>,
    pub geo: GeoData,
    pub types: Vec<(String, Option<String>)>,
    pub logs: Vec<(String, String, Duration)>,
    pub negotiator_proto: String,

    pub verify_ssl: bool,
    pub timeout: i32,
    pub runtimes: Vec<f64>,

    pub tcp_stream: Option<TcpStream>,
    pub tls_stream: Option<TlsStream<TcpStream>>,

    pub request_stat: i32,
    pub error_stat: HashMap<String, i32>,
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
                logs: vec![],
                negotiator_proto: "HTTP".to_string(),
                timeout: 5,
                runtimes: vec![],
                tcp_stream: None,
                tls_stream: None,
                verify_ssl: false,
                request_stat: 0,
                error_stat: HashMap::new(),
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

    pub fn as_text(&self) -> String {
        format!("{}:{}", self.host, self.port)
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

    pub async fn send(&mut self, body: &[u8]) {
        if self.tls_stream.is_some() {
            self.send_tls(&body).await;
        } else if self.tcp_stream.is_some() {
            self.send_tcp(&body).await
        } else {
            log::error!("must connect to the proxy first");
        }
    }

    pub async fn recv(&mut self, size: usize) -> Option<Vec<u8>> {
        if self.tls_stream.is_some() {
            return self.recv_tls(size).await;
        } else if self.tcp_stream.is_some() {
            return self.recv_tcp(size).await;
        } else {
            log::error!("must connect to the proxy first");
        }
        None
    }

    pub async fn recv_all(&mut self) -> Option<Vec<u8>> {
        if self.tls_stream.is_some() {
            return self.recv_all_tls().await;
        } else if self.tcp_stream.is_some() {
            return self.recv_all_tcp().await;
        }
        None
    }

    pub async fn close(&mut self) {
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.shutdown().await {
                Ok(_) => {
                    self.tcp_stream = None;
                    self.log("Connection closed", None, None)
                }
                Err(e) => self.log(
                    format!("Failed to close connection: {}", e).as_str(),
                    None,
                    Some(e.to_string()),
                ),
            }
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

    async fn send_tcp(&mut self, body: &[u8]) {
        let stime = Instant::now();
        let stream = self.tcp_stream.as_mut().unwrap();
        match stream.write_all(body).await {
            Ok(_) => self.log(
                format!("Sending {} bytes", body.len()).as_str(),
                Some(stime.elapsed()),
                None,
            ),
            Err(e) => {
                self.log(
                    format!("Sending error: {}", e).as_str(),
                    Some(stime.elapsed()),
                    Some(e.to_string()),
                );
            }
        };
    }

    async fn recv_tcp(&mut self, size: usize) -> Option<Vec<u8>> {
        let stime = Instant::now();
        let stream = self.tcp_stream.as_mut().unwrap();
        let mut chunk = vec![0; size];
        match timeout(
            Duration::from_secs(self.timeout as u64),
            stream.read_exact(&mut chunk),
        )
        .await
        {
            Ok(buffer) => match buffer {
                Ok(buf_size) => {
                    if buf_size > 0 {
                        self.log(
                            format!("Received {} bytes", buf_size).as_str(),
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
                None,
                Some(e.to_string()),
            ),
        }
        None
    }

    async fn recv_all_tcp(&mut self) -> Option<Vec<u8>> {
        let stime = Instant::now();
        let stream = self.tcp_stream.as_mut().unwrap();
        let mut chunk = vec![0; 1024];
        let mut buf = Vec::new();
        loop {
            match timeout(
                Duration::from_secs(self.timeout as u64),
                stream.read(&mut chunk),
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
                        None,
                        Some(e.to_string()),
                    );
                    // TODO: log error
                    break;
                }
            }
        }
        self.log(
            format!("Received {} bytes", buf.len()).as_str(),
            Some(stime.elapsed()),
            None,
        );
        Some(buf)
    }
}

// TLS / SSL
impl Proxy {
    /// Only used to check the https protocol not for servers.
    pub async fn connect_ssl(&mut self, connect_payload: &[u8]) -> bool {
        if self.tcp_stream.is_some() {
            self.logs.clear();
            self.runtimes.clear();
            self.request_stat = 0;
        }

        let tcp_stream = self.connect_tcp().await;
        if tcp_stream.is_none() {
            return false;
        }

        let mut tcp_stream = tcp_stream.unwrap();

        // send connnect requests
        let stime_send = Instant::now();
        match tcp_stream.write_all(connect_payload).await {
            Ok(_) => self.log(
                format!("Sending {} bytes", connect_payload.len()).as_str(),
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
                            format!("Received {} bytes", buf_size).as_str(),
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
                    None,
                    Some(e.to_string()),
                );
                return false;
            }
        }

        let response = Response::parse(&chunk);
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
                self.log("SSL: Connection timeout", None, Some(e.to_string()));
                None
            }
        };

        self.tls_stream.is_some()
    }

    async fn send_tls(&mut self, body: &[u8]) {
        let stime = Instant::now();
        let stream = self.tls_stream.as_mut().unwrap();
        match stream.write_all(body).await {
            Ok(_) => self.log(
                format!("SSL: Sending {} bytes", body.len()).as_str(),
                Some(stime.elapsed()),
                None,
            ),
            Err(e) => {
                self.log(
                    format!("SSL: Sending error: {}", e).as_str(),
                    Some(stime.elapsed()),
                    Some(e.to_string()),
                );
            }
        };
    }

    async fn recv_tls(&mut self, size: usize) -> Option<Vec<u8>> {
        let stime = Instant::now();
        let stream = self.tls_stream.as_mut().unwrap();
        let mut chunk = vec![0; size];
        match timeout(
            Duration::from_secs(self.timeout as u64),
            stream.read_exact(&mut chunk),
        )
        .await
        {
            Ok(buffer) => match buffer {
                Ok(buf_size) => {
                    if buf_size > 0 {
                        self.log(
                            format!("SSL: Received {} bytes", buf_size).as_str(),
                            Some(stime.elapsed()),
                            None,
                        );
                        return Some(chunk.to_vec());
                    }
                }
                Err(e) => self.log(
                    format!("SSL: Failed to receive: {}", e).as_str(),
                    Some(stime.elapsed()),
                    Some(e.to_string()),
                ),
            },
            Err(e) => self.log(
                format!("SSL: Received timeout: {}", e).as_str(),
                None,
                Some(e.to_string()),
            ),
        }
        None
    }

    async fn recv_all_tls(&mut self) -> Option<Vec<u8>> {
        let stime = Instant::now();
        let stream = self.tls_stream.as_mut().unwrap();
        let mut chunk = vec![0; 512];
        let mut buf = Vec::new();
        loop {
            match timeout(
                Duration::from_secs(self.timeout as u64),
                stream.read(&mut chunk),
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
                            format!("SSL: Failed to receive: {}", e).as_str(),
                            Some(stime.elapsed()),
                            Some(e.to_string()),
                        );
                        break;
                    }
                },

                Err(e) => {
                    self.log(
                        format!("SSL: Received timeout: {}", e).as_str(),
                        None,
                        Some(e.to_string()),
                    );
                    // TODO: log error
                    break;
                }
            }
        }
        self.log(
            format!("SSL: Received {} bytes", buf.len()).as_str(),
            Some(stime.elapsed()),
            None,
        );
        Some(buf)
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

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.port == other.port
    }
}
