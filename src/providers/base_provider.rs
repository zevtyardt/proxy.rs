#[derive(Debug)]
pub struct BaseProvider {
    pub proxies: Vec<String>,
}

impl BaseProvider {
    pub fn new() -> Self {
        BaseProvider { proxies: vec![] }
    }
}
