#[macro_export]
macro_rules! error_context {
    () => {
        concat!("at ", file!(), " line ", line!(), " column ", column!())
    };
}
