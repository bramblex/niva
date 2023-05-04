use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use serde_json::Value;
use url::Url;
use wry::http::HeaderValue;

pub type ArcMut<T> = Arc<Mutex<T>>;

pub fn arc<T>(t: T) -> Arc<T> {
    Arc::new(t)
}

pub fn arc_mut<T>(t: T) -> ArcMut<T> {
    Arc::new(Mutex::new(t))
}

pub struct IdCounter {
    next_id: u16,
}

impl IdCounter {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    pub fn next<T>(&mut self, excludes: &HashMap<u16, T>) -> Result<u16> {
        for _ in 0..u16::MAX {
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
            println!("[Error]: {}", e);
        }
    };
}

#[macro_export]
macro_rules! log {
    ($result:expr) => {
        println!("[Info]: {}", $result);
    };
}

#[macro_export]
macro_rules! log_err {
    ($result:expr) => {
        println!("[Error]: {}", $result);
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

