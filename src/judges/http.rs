use anyhow::Context;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{error_context, utils::random::get_random_element};

use super::{check_all_host, Judge};

lazy_static! {
    static ref HOSTS: Mutex<Vec<Judge>> = Mutex::new(vec![]);
}

pub async fn get_http_judge() -> anyhow::Result<Judge> {
    let hosts = HOSTS.lock().await;
    if hosts.is_empty() {
        anyhow::bail!("hosts is empty, please initiate it first");
    }
    Ok(get_random_element(&hosts)
        .context(error_context!())?
        .clone())
}

pub async fn init_http_judge() -> anyhow::Result<bool> {
    let mut hosts = HOSTS.lock().await;
    if !hosts.is_empty() {
        return Ok(true);
    }
    let urls = vec![
        "http://httpheader.net/azenv.php",
        "http://httpbin.org/get?show_env",
        "http://azenv.net/",
        "http://mojeip.net.pl/asdfa/azenv.php",
        "http://proxyjudge.us",
        "http://pascal.hoez.free.fr/azenv.php",
        "http://www.9ravens.com/env.cgi",
        "http://www3.wind.ne.jp/hassii/env.cgi",
        "http://shinh.org/env.cgi",
        "http://www2t.biglobe.ne.jp/~take52/test/env.cgi",
    ];

    hosts.extend(check_all_host(urls).await.context(error_context!())?);
    Ok(!hosts.is_empty())
}
