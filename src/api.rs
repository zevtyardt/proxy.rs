use crate::{
    judge::{self, Judge},
    proxy::Proxy,
    resolver::Resolver,
    utils::{queue::FifoQueue, CustomFuture},
};

// Pause between grabbing cycles; in seconds.
const GRAB_PAUSE: u32 = 180;

// The maximum number of providers that are parsed concurrently
const MAX_CONCURRENT_PROVIDERS: u16 = 3;

pub struct ProxyRs {
    pub proxies: FifoQueue<Proxy>,
    pub resolver: Resolver,
    pub timeout: i32,
    pub verify_ssl: bool,
    pub unique_proxies: Vec<Proxy>,
    pub all_tasks: Vec<CustomFuture<()>>,
    pub limit: i32,
    pub countries: Vec<String>,
    pub on_check: FifoQueue<Proxy>,
    pub max_tries: i32,
    pub judges: Vec<Judge>,
}

impl ProxyRs {
    pub async fn get_judges(&mut self) {
        self.judges = judge::get_judges(self.verify_ssl).await;
    }

    pub fn grab(&mut self, countries: Option<Vec<String>>, limit: Option<i32>) {
        if countries.is_some() {
            self.countries = countries.unwrap()
        }
        if limit.is_some() {
            self.limit = limit.unwrap()
        }

        self.all_tasks.push(Box::pin(async {}));
    }
}

impl ProxyRs {
    pub fn set_timeout(&mut self, value: i32) {
        self.timeout = value;
    }

    pub fn set_limit(&mut self, value: i32) {
        self.limit = value;
    }

    pub fn set_verify_ssl(&mut self, value: bool) {
        self.verify_ssl = value;
    }

    pub fn set_max_tries(&mut self, value: i32) {
        self.max_tries = value;
    }
}

impl Default for ProxyRs {
    fn default() -> Self {
        ProxyRs {
            proxies: FifoQueue::new(),
            resolver: Resolver::new(),
            timeout: 8,
            verify_ssl: false,
            unique_proxies: vec![],
            all_tasks: vec![],
            limit: 0,
            countries: vec![],
            on_check: FifoQueue::new(),
            max_tries: 3,
            judges: vec![],
        }
    }
}
