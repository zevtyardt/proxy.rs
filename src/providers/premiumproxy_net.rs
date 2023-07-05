use crate::{providers::base_provider::BaseProvider, utils::vec_of_strings};

#[derive(Debug, Clone)]
pub struct PremiumproxyNetProvider {
    pub base: BaseProvider,
    pub url: String,
    pub pattern: String,
}

impl PremiumproxyNetProvider {
    pub async fn get_proxies(&mut self) -> Vec<(String, u16, Vec<String>)> {
        let req = self.base.build_get_request(self.url.clone());
        let html = self.base.get_html(req).await;
        let proxies = self.base.find_proxies(self.pattern.clone(), html.as_str());
        self.base.update_stack(&proxies).await;

        proxies
    }
}

impl Default for PremiumproxyNetProvider {
    fn default() -> Self {
        Self {
            base: BaseProvider {
                proto: vec_of_strings![
                    "HTTP",
                    "CONNECT:80",
                    "HTTPS",
                    "CONNECT:25",
                    "SOCKS4",
                    "SOCKS5"
                ],
                domain: "premiumproxy.net".to_string(),
                ..Default::default()
            },
            url: "https://premiumproxy.net/full-proxy-list".to_string(),
            pattern:
                r#"<font.*?>\s*(?P<ip>(?:\d+\.?){4})\s*<font.*?>\s*\:\s*</font>\s*(?P<port>\d+)"#
                    .to_string(),
        }
    }
}
