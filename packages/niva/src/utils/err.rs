
#[macro_export]
macro_rules! err {
    ($fmt:expr $(, $arg:expr)*) => {
        Err(anyhow::anyhow!(concat!("File: {} Line: {}: ", $fmt), file!(), line!() $(, $arg)*))
    };
}