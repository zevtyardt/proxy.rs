use super::base_provider::BaseProvider;
use crate::utils::vec_of_strings;

#[derive(Debug, Clone)]
pub struct FreeProxyListNetProvider {
    pub base: BaseProvider,
    pub url: String,
    pub pattern: String,
}

impl FreeProxyListNetProvider {
    pub async fn get_proxies(&mut self) -> Vec<(String, u16, Vec<String>)> {
        let req = self.base.build_get_request(self.url.clone());
        let html = self.base.get_html(req).await;
        let proxies = self.base.find_proxies(self.pattern.clone(), html.as_str());
        self.base.update_stack(&proxies).await;

        proxies
    }
}

impl Default for FreeProxyListNetProvider {
    fn default() -> Self {
        Self {
            base: BaseProvider {
                proto: vec_of_strings!["HTTP", "CONNECT:80", "HTTPS", "CONNECT:25"],
                domain: "free-proxy-list.net".to_string(),
                ..Default::default()
            },
            url: "https://free-proxy-list.net/".to_string(),
            pattern: r#"(?P<ip>(?:\d+\.?){4})\:(?P<port>\d+)"#.to_string(),
        }
    }
}
