use std::{collections::HashMap, net::IpAddr, time::Duration};

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
    pub logs: Vec<(String, String, Duration)>,
    pub negotiator_proto: String,
    pub timeout: i32,
    pub runtimes: Vec<f64>,

    pub stream: Option<TcpStream>,
    pub request_stat: i32,
    pub error_stat: HashMap<String, i32>,
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
            request_stat: 0,
            error_stat: HashMap::new(),
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

        self.logs.push((
            self.negotiator_proto.clone(),
            msg.to_string(),
            stime.unwrap(),
        ));

        let runtime_sec = stime.unwrap().as_secs_f64();
        self.runtimes.push(runtime_sec)
    }

    //    pub fn log_error(&mut self) {}

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
                    self.request_stat += 1;
                    Some(stream)
                }
                Err(e) => {
                    self.log(
                        format!("Connection error: {}", e).as_str(),
                        Some(stime.elapsed()),
                    );
                    None
                }
            },
            Err(_) => {
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
                self.log(
                    format!("Sending error: {}", e).as_str(),
                    Some(stime.elapsed()),
                );
                // TODO: Log error
            }
        };
    }

    pub async fn recv_all(&mut self) -> Option<Vec<u8>> {
        if self.stream.is_none() {
            log::debug!("Please run .connect() first");
            return None;
        }

        let stime = Instant::now();
        let stream = self.stream.as_mut().unwrap();
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

    pub async fn close(&mut self) {
        if let Some(stream) = self.stream.as_mut() {
            match stream.shutdown().await {
                Ok(_) => {
                    self.stream = None;
                    self.log("Connection closed", None)
                }
                Err(e) => {
                    log::debug!("{}", e);
                }
            }
        }
    }
}

// TODO ADD TYPES
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

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}
