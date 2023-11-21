use anyhow::Context;
use lazy_static::lazy_static;
use tokio::sync::Mutex;
use url::Url;

use crate::{error_context, utils::random::get_random_element};

use super::check_all_host;

lazy_static! {
    static ref HOSTS: Mutex<Vec<Url>> = Mutex::new(vec![]);
}

pub async fn get_https_judge() -> anyhow::Result<Url> {
    let hosts = HOSTS.lock().await;
    if hosts.is_empty() {
        anyhow::bail!("hosts is empty, please initiate it first");
    }

    Ok(get_random_element(&hosts)
        .context(error_context!())?
        .clone())
}

pub async fn init_https_judge() -> anyhow::Result<bool> {
    let mut hosts = HOSTS.lock().await;
    if !hosts.is_empty() {
        return Ok(true);
    }
    let urls = vec![
        "https://httpbin.org/get?show_env",
        "https://www.proxy-listen.de/azenv.php",
        "https://httpheader.net/azenv.php",
        "https://www.proxyjudge.info",
    ];

    hosts.extend(check_all_host(urls).await.context(error_context!())?);
    Ok(!hosts.is_empty())
}
