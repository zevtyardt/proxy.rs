use std::{collections::HashMap, net::IpAddr, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Instant},
};

use crate::resolver::{GeoData, Resolver};

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
                stream: None,
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
        let stime = Instant::now();
        self.log("Initial connection", Some(stime.elapsed()), None);
        self.stream = match timeout(
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
                self.log(
                    "Connection timeout",
                    Some(stime.elapsed()),
                    Some(e.to_string()),
                );
                None
            }
        };

        self.stream.is_some()
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
            format!("Received {} bytes", buf.len()).as_str(),
            Some(stime.elapsed()),
            None,
        );
        Some(buf)
    }

    pub async fn close(&mut self) {
        if let Some(stream) = self.stream.as_mut() {
            match stream.shutdown().await {
                Ok(_) => {
                    self.stream = None;
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
}
