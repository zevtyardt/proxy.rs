use std::{net::IpAddr, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Instant},
};

use crate::{
    negotiators::{http_negotiator::HttpNegotiator, Negotiators},
    resolver::{GeoData, Resolver},
};

#[derive(Debug)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub expected_types: Vec<String>,
    pub geo: GeoData,
    pub types: Vec<(String, Option<String>)>,
    pub logs: Vec<(String, Duration)>,
    pub negotiator_proto: String,
    pub timeout: i32,
    pub runtimes: Vec<f64>,

    pub stream: Option<TcpStream>,
}

impl Proxy {
    pub async fn create(host: &str, port: u16, expected_types: Vec<String>) -> Self {
        let mut host = host.to_string();
        let resolver = Resolver::new();
        if !resolver.host_is_ip(&host) {
            host = resolver.resolve(host).await;
        }
        let geo = resolver.get_ip_info(host.parse::<IpAddr>().unwrap()).await;
        Proxy {
            host,
            port,
            expected_types,
            geo,
            types: vec![],
            logs: vec![],
            negotiator_proto: "HTTP".to_string(),
            timeout: 8,
            runtimes: vec![],
            stream: None,
        }
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
    pub fn log(&mut self, msg: &str, mut stime: Option<Duration>) {
        if stime.is_none() {
            stime = Some(Duration::from_micros(0))
        }
        log::debug!(
            "{}:{} [{}] {}, Runtime {:?}",
            self.host,
            self.port,
            self.negotiator_proto,
            msg,
            stime.unwrap()
        );

        self.logs.push((msg.to_string(), stime.unwrap()));
    }

    pub fn negotiator(&self) -> Box<dyn Negotiators> {
        // TODO ADD HTTPS, SOCKS4, SOCKS5
        //
        // HTTP is default negotiator
        Box::new(HttpNegotiator::default())
    }

    pub async fn connect(&mut self) {
        let stime = Instant::now();
        self.log("Initial connection", Some(stime.elapsed()));
        self.stream = match timeout(
            Duration::from_secs(self.timeout as u64),
            TcpStream::connect(self.as_text()),
        )
        .await
        {
            Ok(stream) => match stream {
                Ok(stream) => {
                    self.log("Connection success", Some(stime.elapsed()));
                    Some(stream)
                }
                Err(e) => {
                    log::debug!("{}", e);
                    self.log("Connection error", Some(stime.elapsed()));
                    None
                }
            },
            Err(e) => {
                log::debug!("{}", e);
                self.log("Connection timeout", Some(stime.elapsed()));
                None
            }
        };
    }

    pub fn connect_ssl(&self) {
        unimplemented!()
    }

    pub async fn send(&mut self, body: &[u8]) {
        if self.stream.is_none() {
            log::debug!("Please run .connect() first!");
            return;
        }
        let stime = Instant::now();
        let stream = self.stream.as_mut().unwrap();
        match stream.write_all(body).await {
            Ok(_) => self.log(
                format!("Sending {} bytes", body.len()).as_str(),
                Some(stime.elapsed()),
            ),
            Err(e) => {
                log::debug!("{}", e);
                // TODO: Log error
            }
        };
    }

    pub async fn recv(&mut self) -> Option<String> {
        if self.stream.is_none() {
            log::debug!("Please run .connect() first");
            return None;
        }
        let stime = Instant::now();
        let stream = self.stream.as_mut().unwrap();
        let mut chunk = vec![0; 1024];
        let mut buf = String::new();

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
                        let text = String::from_utf8_lossy(&chunk[0..buf_size]);
                        buf.push_str(&text);
                    }
                    Err(e) => {
                        log::debug!("{}", e);
                        break;
                    }
                },

                Err(_) => {
                    log::debug!("Read timeout");
                    // TODO: log error
                    break;
                }
            }
        }
        self.log(
            format!("Received {} bytes", buf.len()).as_str(),
            Some(stime.elapsed()),
        );
        Some(buf)
    }

    pub async fn close(&self) {
        unimplemented!()
    }
}

// TODO ADD TYPES
impl std::fmt::Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Proxy {} {:.2}s {}:{}>",
            self.geo.iso_code,
            self.avg_resp_time(),
            self.host,
            self.port
        )
    }
}

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.port == other.port
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}
