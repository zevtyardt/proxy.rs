use super::base_provider::BaseProvider;
use crate::{proxy::Proxy, utils::vec_of_strings};
use regex::Regex;

#[derive(Debug)]
pub struct FreeProxyListNetProvider {
    pub base: BaseProvider,
    pub url: String,
    pub pattern: String,
}

impl FreeProxyListNetProvider {
    pub async fn get_proxies(&mut self) -> Vec<(String, u16, Vec<String>)> {
        self.base.start();

        let req = self.base.client.get(self.url.clone());
        let html = self.base.get_html(req).await;
        let proxies = self.base.find_proxies(self.pattern.clone(), html.as_str());
        self.base.update_stack(&proxies).await;

        proxies
    }
}

impl Default for FreeProxyListNetProvider {
    fn default() -> Self {
        let mut base_provider = BaseProvider::default();
        base_provider.proto = vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"];
        base_provider.domain = "free-proxy-list.net".to_string();

        FreeProxyListNetProvider {
            base: base_provider,
            url: "https://free-proxy-list.net/".to_string(),
            pattern: r#"(?P<ip>(?:\d+\.?){4})\:(?P<port>\d+)"#.to_string(),
        }
    }
}
