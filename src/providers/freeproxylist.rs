use super::base_provider::BaseProvider;
use crate::proxy::Proxy;
use regex::Regex;

#[derive(Debug)]
pub struct FreeProxyListNetProvider<'a> {
    pub base: BaseProvider<'a>,
    pub url: &'a str,
    pub pattern: &'a str,
}

impl FreeProxyListNetProvider<'_> {
    pub async fn get_proxies(&self) -> Vec<(String, u16, Vec<&str>)> {
        let req = self.base.client.get(self.url);
        let html = self.base.get_html(req).await;
        self.base.find_proxies(self.pattern, html.as_str())
    }
}

impl Default for FreeProxyListNetProvider<'_> {
    fn default() -> Self {
        let mut base_provider = BaseProvider::default();
        base_provider.proto = vec!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"];

        FreeProxyListNetProvider {
            base: base_provider,
            url: "https://free-proxy-list.net/",
            pattern: r#"(?P<ip>(?:\d+\.?){4})\:(?P<port>\d+)"#,
        }
    }
}
