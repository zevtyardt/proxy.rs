use crate::{judge::Judge, proxy::Proxy};

#[derive(Debug, Clone)]
pub struct HttpsNegotiator {
    pub name: String,
    pub check_anon_lvl: bool,
    pub use_full_path: bool,
}

impl HttpsNegotiator {
    pub async fn negotiate(&self, proxy: &mut Proxy, judge: &Judge) -> bool {
        let connect_payload = format!(
            "CONNECT {0}:443 HTTP/1.1\r\nHost: {0}\r\nConnection: keep-alive\r\n\r\n",
            judge.host
        );
        proxy.connect_ssl(connect_payload.as_bytes()).await
    }
}

impl Default for HttpsNegotiator {
    fn default() -> Self {
        Self {
            name: "HTTPS".to_string(),
            check_anon_lvl: false,
            use_full_path: false,
        }
    }
}
