use dirs::{data_dir, data_local_dir};
use std::path::PathBuf;

pub mod logger;

pub fn get_data_dir(file: Option<&str>) -> PathBuf {
    let mut path = if let Some(path) = data_dir() {
        path
    } else if let Some(path) = data_local_dir() {
        path
    } else {
        PathBuf::from("./")
    };
    path.push("proxy-rs/");
    if let Some(file) = file {
        path.push(file);
    }
    path
}
