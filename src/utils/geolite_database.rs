use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use hyper::{body::HttpBody, header::CONTENT_LENGTH, Body, Request};
use indicatif::{ProgressBar, ProgressStyle};
use maxminddb::Reader;
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};

use super::http::{hyper_client, random_useragent};

const GEOLITEDB: &str = "GeoLite2-City.mmdb";
const GEOLITEDB_DOWNLOAD_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/data/GeoLite2-City.mmdb";
const GEOLITEDB_CHECKSUM_URL: &str =
    "https://raw.githubusercontent.com/zevtyardt/proxy.rs/main/data/Geolite2-City.mmdb.checksum";

async fn download_geolite_db() {
    let bar = ProgressBar::new(0);
    bar.set_style(
        ProgressStyle::with_template(
            "INFO  Downloading GeoLite2-City.mmdb => {percent}% {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
        )
        .unwrap(),
    );

    let client = hyper_client();
    let request = Request::builder()
        .uri(GEOLITEDB_DOWNLOAD_URL)
        .body(Body::empty())
        .unwrap();

    let local_db = PathBuf::from(format!("./data/{}", GEOLITEDB));
    if let Ok(mut file) = File::create(local_db).await {
        if let Ok(response) = client.request(request).await {
            let headers = response.headers();
            if let Some(content_length) = headers.get(CONTENT_LENGTH) {
                if let Ok(content_length) = content_length.to_str() {
                    bar.set_length(content_length.parse::<u64>().unwrap());
                }
            }

            let mut body = response.into_body();
            while let Some(Ok(bytes)) = body.data().await {
                if file.write_all(&bytes).await.is_ok() {
                    bar.inc(bytes.len() as u64)
                }
            }
        }
    }
    bar.finish();
}

async fn calculate_checksum(file_path: &Path) -> String {
    let f = File::open(file_path).await.unwrap();
    let len = f.metadata().await.unwrap().len();

    let buf_len = len.min(1_000_000) as usize;
    let mut buf = BufReader::with_capacity(buf_len, f);
    let mut context = md5::Context::new();

    loop {
        let part = buf.fill_buf().await.unwrap();
        if part.is_empty() {
            break;
        }
        context.consume(part);
        let part_len = part.len();
        buf.consume(part_len);
    }
    let digest = context.compute();
    format!("{:x}", digest)
}

pub async fn open_geolite_db() -> Option<Reader<Vec<u8>>> {
    if let Some(project_dir) =
        ProjectDirs::from_path(option_env!("CARGO_PKG_NAME").unwrap_or("proxy-rs").into())
    {
        let data_dir = project_dir.data_dir().to_path_buf();
        let mut warn = false;
        loop {
            let db = data_dir.join("data").join(GEOLITEDB);

            if let Some(db_parent) = &db.parent() {
                if !db_parent.exists() {
                    fs::create_dir_all(db_parent).await.unwrap();
                }
            }

            let mut redownload = true;
            if db.exists() {
                let client = hyper_client();
                let request = Request::builder()
                    .header("User-Agent", random_useragent(true))
                    .uri(GEOLITEDB_CHECKSUM_URL)
                    .body(Body::empty())
                    .unwrap();

                if let Ok(response) = client.request(request).await {
                    if let Ok(body) = hyper::body::to_bytes(response.into_body()).await {
                        let expected_checksum = String::from_utf8_lossy(&body).trim().to_string();
                        let checksum = calculate_checksum(&db).await;
                        redownload = !expected_checksum.eq(&checksum);

                        if redownload && !warn {
                            warn = true;
                            log::warn!("Database checksum is different. Re-downloading..")
                        }
                    }
                }
            }

            if redownload {
                let local_db = PathBuf::from(format!("./data/{}", GEOLITEDB));
                if let Some(local_db_parent) = &local_db.parent() {
                    if !local_db_parent.exists() {
                        fs::create_dir(local_db_parent).await.unwrap()
                    }
                }

                if !local_db.exists() {
                    download_geolite_db().await;
                }

                fs::copy(&local_db, &db).await.unwrap();
                fs::remove_dir_all(local_db.parent().unwrap())
                    .await
                    .unwrap();
            }

            match Reader::open_readfile(&db) {
                Ok(database) => return Some(database),
                Err(e) => log::debug!("{} retrying..", e),
            }
        }
    }
    None
}
