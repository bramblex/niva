pub mod json;
pub mod path;
pub mod url;
pub mod macros;

#[cfg(target_os = "windows")]
pub mod win;

#[cfg(target_os = "macos")]
pub mod mac;