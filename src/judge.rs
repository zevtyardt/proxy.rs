use url::Url;

#[derive(Debug)]
pub struct Judge {
    pub url: Url,
    pub http: bool,
    pub https: bool,
    pub smtp: bool,
}

impl Judge {
    pub fn new(url: &str) -> Self {
        let url = Url::parse(url).unwrap();
        Judge {
            url,
            http: false,
            https: false,
            smtp: false,
        }
    }
}
