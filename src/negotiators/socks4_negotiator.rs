use std::{io::Cursor, net::Ipv4Addr};

use byteorder::BigEndian;
use byteorder_pack::PackTo;
use tokio::io::AsyncReadExt;

use crate::proxy::Proxy;

#[derive(Debug, Clone)]
pub struct Socks4Negotiator {
    pub name: String,
    pub check_anon_lvl: bool,
    pub use_full_path: bool,
}

impl Socks4Negotiator {
    pub async fn negotiate(&self, proxy: &mut Proxy) -> bool {
        let bip = proxy.host.parse::<Ipv4Addr>();
        if bip.is_err() {
            return false;
        }

        let data = (4u8, 1u8, proxy.port, bip.unwrap().octets(), 0u8);
        let mut buf = Cursor::new(Vec::new());
        if data.pack_to::<BigEndian, _>(&mut buf).is_err() {
            return false;
        }
        let payload = buf.into_inner();

        proxy.send(payload.as_slice()).await;

        if let Some(data) = proxy.recv(8).await {
            let mut data = data.as_slice();

            let version = data.read_u8().await;
            if version.is_err() || version.unwrap() != 0 {
                proxy.log(
                    "Invalid response version",
                    None,
                    Some("invalid_response_version".to_string()),
                );
                return false;
            }

            let resp = data.read_u8().await;
            if resp.is_err() || resp.unwrap() != 90 {
                proxy.log(
                    "Request rejected or Failed",
                    None,
                    Some("request_failed".to_string()),
                );
                return false;
            }

            proxy.log("Request is granted", None, None);
            return true;
        }
        false
    }
}

impl Default for Socks4Negotiator {
    fn default() -> Self {
        Self {
            name: "SOCKS4".to_string(),
            check_anon_lvl: false,
            use_full_path: false,
        }
    }
}
