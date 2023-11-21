use anyhow::Context;
use lazy_static::lazy_static;
use url::Url;

use crate::{error_context, utils::random::get_random_element};

lazy_static! {
    static ref HOSTS: Vec<Url> = {
        let v = ["smtp://smtp.gmail.com", "smtp://aspmx.l.google.com"];
        v.iter()
            .map(|i| Url::parse(i).context(error_context!()).unwrap())
            .collect::<Vec<Url>>()
    };
}

pub async fn get_smtp_judge() -> anyhow::Result<Url> {
    Ok(get_random_element(&HOSTS)
        .context(error_context!())?
        .clone())
}

pub async fn init_smtp_judge() -> anyhow::Result<bool> {
    Ok(true)
}
