#[macro_export]
macro_rules! debug_error {
    () => {
        concat!("at ", file!(), " line ", line!(), " column ", column!())
    };
}
