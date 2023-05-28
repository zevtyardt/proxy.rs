#[derive(Debug, Clone)]
pub struct HttpNegotiator {
    pub name: String,
    pub check_anon_lvl: bool,
    pub use_full_path: bool,
}

impl HttpNegotiator {
    pub async fn negotiate(&self) -> bool {
        true
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
