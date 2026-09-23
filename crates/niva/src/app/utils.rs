use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Result, anyhow};
use serde_json::Value;

pub type ArcMut<T> = Arc<Mutex<T>>;

static STDIO_MODE: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_stdio_mode(enabled: bool) {
    STDIO_MODE.store(enabled, Ordering::Relaxed);
}

pub(crate) fn log_output(args: std::fmt::Arguments<'_>) {
    if STDIO_MODE.load(Ordering::Relaxed) {
        eprintln!("{args}");
    } else {
        println!("{args}");
    }
}

pub fn arc<T>(t: T) -> Arc<T> {
    Arc::new(t)
}

pub fn arc_mut<T>(t: T) -> ArcMut<T> {
    Arc::new(Mutex::new(t))
}

pub struct IdCounter {
    next_id: u8,
}

impl IdCounter {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    pub fn next<T>(&mut self, excludes: &HashMap<u8, T>) -> Result<u8> {
        for _ in 0..u8::MAX {
            let id = self.next_id;
            if excludes.contains_key(&id) {
                self.next_id += 1;
                continue;
            }
            return Ok(id);
        }
        Err(anyhow!("Failed to find a valid id."))
    }
}

#[macro_export]
macro_rules! unsafe_impl_sync_send {
    ($type:ty) => {
        unsafe impl Send for $type {}
        unsafe impl Sync for $type {}
    };
}

#[macro_export]
macro_rules! set_property_some {
    ($builder:ident, $property:ident, &$value:expr) => {
        if let Some(value) = &$value {
            $builder = $builder.$property(value);
        }
    };
    ($builder:ident, $property:ident, $value:expr) => {
        if let Some(value) = $value {
            $builder = $builder.$property(value.clone());
        }
    };
}

#[macro_export]
macro_rules! set_property {
    ($builder:ident, $property:ident, $value:expr) => {
        $builder = $builder.$property($value);
    };
}

#[macro_export]
macro_rules! lock {
    ($value:expr) => {
        $value
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock {}.", stringify!($value)))
    };
}

#[macro_export]
macro_rules! lock_force {
    ($value:expr) => {
        $value.lock().unwrap()
    };
}

#[macro_export]
macro_rules! logical {
    ($window:expr, $method:ident) => {
        $window.$method().to_logical::<f64>($window.scale_factor())
    };

    ($window:expr, $item:expr, $method:ident) => {
        $item.$method().to_logical::<f64>($window.scale_factor())
    };
}

#[macro_export]
macro_rules! logical_try {
    ($window:expr, $method:ident) => {
        $window.$method()?.to_logical::<f64>($window.scale_factor())
    };
}

#[macro_export]
macro_rules! log_if_err {
    ($result:expr) => {
        if let Err(e) = $result {
            $crate::app::utils::log_output(format_args!("[Error]: {}", e));
        }
    };
}

#[macro_export]
macro_rules! log {
    ($result:expr) => {
        $crate::app::utils::log_output(format_args!("[Info]: {}", $result));
    };
}

#[macro_export]
macro_rules! log_err {
    ($result:expr) => {
        $crate::app::utils::log_output(format_args!("[Error]: {}", $result));
    };
}

pub fn merge_values(dest: Value, src: Value) -> Value {
    match (dest, src) {
        (Value::Null, src) => src,
        (dest, Value::Null) => dest,
        (Value::Object(mut dest_map), Value::Object(src_map)) => {
            for (key, src_val) in src_map {
                let dest_val = dest_map.entry(key).or_insert(Value::Null);
                *dest_val = merge_values(dest_val.take(), src_val);
            }
            Value::Object(dest_map)
        }
        (_, src) => src,
    }
}

// pub fn try_or_log_err<F, T>(mut func: F) where F: FnMut() -> Result<T> {
//     match func() {
//         Ok(_) => {}
//         Err(e) => {
//             log_err!(e);
//         }
//     }
// }

#[macro_export]
macro_rules! try_or_log_err {
    ($body:block ) => {
        match (move || -> anyhow::Result<()> { $body })() {
            Ok(_) => {}
            Err(e) => {
                crate::log_err!(e);
            }
        }
    };
}

pub fn url_join(left: &str, right: &str) -> String {
    if right.is_empty() {
        left.to_string()
    } else if left.ends_with("/") {
        format!("{}{}", left, right)
    } else {
        format!("{}/{}", left, right)
    }
}

pub fn merge_id(window_id: u8, item_id: u8) -> u16 {
    ((window_id as u16) << 8) | (item_id as u16)
}

pub fn split_id(merged_id: u16) -> (u8, u8) {
    ((merged_id >> 8) as u8, merged_id as u8)
}

pub(crate) fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render a debuggable HTML error page instead of a bare status text, so a
/// missing entry/resource shows *what* is missing right in the window.
/// Returns (status, html body) so both the wry and axum layers can wrap it.
pub(crate) fn error_page_html(status: u16, title: &str, uri: &str, detail: &str) -> (u16, Vec<u8>) {
    let body = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
        <title>{status} {title}</title></head><body>\
        <h1>{status} {title}</h1>\
        <p>URI: <code>{}</code></p>\
        <p>{}</p>\
        <p style=\"color:#888\">Niva: check that the entry file \
        exists in the resource directory.</p>\
        </body></html>",
        html_escape(uri),
        html_escape(detail),
    );
    (status, body.into_bytes())
}

/// Run blocking work on smol's elastic thread pool and await it.
/// Rule: NOTHING blocking may run directly in an async API body or on a
/// driver thread — wrap every syscall, subprocess wait and modal loop here.
/// Unlike the old fixed 4-worker pool, this pool grows on demand, so slow
/// calls can never starve other requests (timeouts still bound them).
/// Run blocking work on smol's elastic thread pool and await it.
/// Rule: NOTHING blocking may run directly in an async API body or on a
/// driver thread — wrap every syscall, subprocess wait and modal loop here.
/// Unlike the old fixed 4-worker pool, this pool grows on demand, so slow
/// calls can never starve other requests (timeouts still bound them).
/// The closure always returns `anyhow::Result`; bodies end with `Ok(..)`.
#[macro_export]
macro_rules! blocking {
    ($body:expr) => {
        smol::unblock(move || -> anyhow::Result<_> { $body })
    };
}
