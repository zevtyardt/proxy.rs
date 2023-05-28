pub mod geolite_database;
pub mod http;

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

use futures_util::{stream, StreamExt};
use tokio::task::JoinHandle;
pub(crate) use vec_of_strings;

pub async fn run_parallel<T>(
    tasks: Vec<JoinHandle<T>>,
    mut num_concurrent: Option<usize>,
) -> Vec<Option<T>> {
    if num_concurrent.is_none() {
        num_concurrent = Some(tasks.len());
    }

    let stream = stream::iter(tasks)
        .map(|task| async {
            if let Ok(value) = task.await {
                return Some(value);
            }
            None
        })
        .buffer_unordered(num_concurrent.unwrap());
    stream.collect().await
}
