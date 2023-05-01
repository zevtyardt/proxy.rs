use super::base_provider::BaseProvider;

#[derive(Debug)]
pub struct FreeProxyListNetProvider {
    pub base_provider: BaseProvider,
    pub url: String,
}

impl FreeProxyListNetProvider {
    pub fn new() -> Self {
        FreeProxyListNetProvider {
            base_provider: BaseProvider::new(),
            url: "https://www.freeproxylists.net".to_string(),
        }
    }

    pub fn add(&mut self, proxy: &str) {
        self.base_provider.proxies.push(proxy.to_string())
    }
}
