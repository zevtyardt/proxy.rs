use crate::{judge::Judge, proxy::Proxy, utils::http::response::ResponseParser};

#[derive(Debug, Clone)]
pub struct Connect80Negotiator {
    pub name: String,
    pub check_anon_lvl: bool,
    pub use_full_path: bool,
}

impl Connect80Negotiator {
    pub async fn negotiate(&self, proxy: &mut Proxy, judge: &Judge) -> bool {
        let connect_payload = format!(
            "CONNECT {0}:80 HTTP/1.1\r\nHost: {0}\r\nConnection: keep-alive\r\n\r\n",
            judge.host
        );
        proxy.send(connect_payload.as_bytes()).await;

        if let Some(data) = proxy.recv_all().await {
            let response = ResponseParser::parse(data.as_slice());
            if let Some(status_code) = response.status_code {
                if status_code == 200 {
                    return true;
                }
                proxy.log(
                    format!("Connect: failed. HTTP status: {}", status_code).as_str(),
                    None,
                    Some("bad_status_error".to_string()),
                );
            }
        }
        false
    }
}

impl Default for Connect80Negotiator {
    fn default() -> Self {
        Self {
            name: "connect:80".to_string(),
            check_anon_lvl: false,
            use_full_path: false,
        }
    }
}
