pub mod json;

#[cfg(target_os = "windows")]
pub mod win;

#[cfg(target_os = "macos")]
pub mod mac;