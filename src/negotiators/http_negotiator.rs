use tokio::task::JoinHandle;

use super::Negotiators;

#[derive(Debug, Clone)]
pub struct HttpNegotiator {
    name: String,
    check_anon_lvl: bool,
    use_full_path: bool,
}

impl Negotiators for HttpNegotiator {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn check_anon_lvl(&self) -> bool {
        self.check_anon_lvl
    }

    fn use_full_path(&self) -> bool {
        self.use_full_path
    }

    fn negotiate(&self, _host: &String, _ip: &String) -> JoinHandle<bool> {
        tokio::task::spawn(async { true })
    }
}

impl Default for HttpNegotiator {
    fn default() -> Self {
        Self {
            name: "HTTP".to_string(),
            check_anon_lvl: true,
            use_full_path: true,
        }
    }
}
