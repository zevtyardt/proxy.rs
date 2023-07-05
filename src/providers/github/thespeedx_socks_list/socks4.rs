use crate::{providers::base_provider::BaseProvider, utils::vec_of_strings};

#[derive(Debug, Clone)]
pub struct GithubTheSpeedXProxyListSocks4Provider {
    pub base: BaseProvider,
    pub url: String,
    pub pattern: String,
}

impl GithubTheSpeedXProxyListSocks4Provider {
    pub async fn get_proxies(&mut self) -> Vec<(String, u16, Vec<String>)> {
        let req = self.base.build_get_request(self.url.clone());
        let html = self.base.get_html(req).await;
        let proxies = self.base.find_proxies(self.pattern.clone(), html.as_str());
        self.base.update_stack(&proxies).await;

        proxies
    }
}

impl Default for GithubTheSpeedXProxyListSocks4Provider {
    fn default() -> Self {
        Self {
            base: BaseProvider {
                proto: vec_of_strings!["SOCKS4"],
                domain: "TheSpeedX/SOCKS-List/socks4".to_string(),
                ..Default::default()
            },
            url: "https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/socks4.txt"
                .to_string(),
            pattern: r#"(?P<ip>(?:\d+\.?){4})\:(?P<port>\d+)"#.to_string(),
        }
    }
}
