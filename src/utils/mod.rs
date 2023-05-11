use futures_util::{stream, StreamExt};

pub mod geolite_database;
pub mod http;
pub mod queue;

pub type CustomFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T>>>;

/* Additional functions to run future tasks in parallel */
pub async fn run_parallel<T>(tasks: Vec<CustomFuture<T>>, num_concurrent: usize) -> Vec<T> {
    stream::iter(tasks)
        .buffer_unordered(num_concurrent)
        .collect::<Vec<T>>()
        .await
}

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub(crate) use vec_of_strings;
