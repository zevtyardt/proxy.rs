use tokio::task::JoinHandle;

pub mod http_negotiator;

pub trait Negotiators {
    fn name(&self) -> String;
    fn check_anon_lvl(&self) -> bool;
    fn use_full_path(&self) -> bool;
    fn negotiate(&self, host: &String, ip: &String) -> JoinHandle<bool>;
}
