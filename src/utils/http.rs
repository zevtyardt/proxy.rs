use rand::Rng;

pub fn random_useragent() -> String {
    let mut rng = rand::thread_rng();
    let name = option_env!("CARGO_PKG_NAME").unwrap_or("proxyrs");
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");
    let rv = rng.gen_range(1000..9999);

    format!("{}/{}/{}", name, version, rv)
}
