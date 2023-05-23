use crate::{providers::base_provider::BaseProvider, utils::vec_of_strings};

#[derive(Debug, Clone)]
pub struct ProxyscanIoSocks4Provider {
    pub base: BaseProvider,
    pub url: String,
    pub pattern: String,
}

impl ProxyscanIoSocks4Provider {
    pub async fn get_proxies(&mut self) -> Vec<(String, u16, Vec<String>)> {
        self.base.start();

        let req = self.base.client.get(self.url.clone());
        let html = self.base.get_html(req).await;
        let proxies = self.base.find_proxies(self.pattern.clone(), html.as_str());
        self.base.update_stack(&proxies).await;

        proxies
    }
}

impl Default for ProxyscanIoSocks4Provider {
    fn default() -> Self {
        Self {
            base: BaseProvider {
                proto: vec_of_strings!["SOCKS4"],
                domain: "proxyscan.io/socks4".to_string(),
                ..Default::default()
            },
            url: "https://www.proxyscan.io/download?type=socks4".to_string(),
            pattern: r#"(?P<ip>(?:\d+\.?){4})\:(?P<port>\d+)"#.to_string(),
        }
    }
}
