pub mod geolite_database;
pub mod http;
pub mod serializer;
pub mod update;

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub(crate) use vec_of_strings;
