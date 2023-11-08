use std::path::PathBuf;

use dirs::{data_dir, data_local_dir};

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
