use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use url::Url;
use wry::http::HeaderValue;

pub type ArcMut<T> = Arc<Mutex<T>>;

pub fn arc<T>(t: T) -> Arc<T> {
    Arc::new(t)
}

pub fn arc_mut<T>(t: T) -> ArcMut<T> {
    Arc::new(Mutex::new(t))
}

pub struct Counter<T> {
    next_id: T,
}

impl Counter<u32> {
    pub fn new(start: u32) -> Self {
        Self { next_id: start }
    }

    pub fn next(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl Counter<u16> {
    pub fn new(start: u16) -> Self {
        Self { next_id: start }
    }

    pub fn next(&mut self) -> u16 {
        let id = self.next_id;
        self.next_id += 1;
        id
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
    ($builder:ident, $property:ident, $value:expr) => {
        if let Some(value) = $value.clone() {
            $builder = $builder.$property(value);
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

#[macro_export]
macro_rules! opts_match {
    ($request:expr, $id0:ident:$arg0:ty) => {
        let ($id0,) = $request.args().optional::<($arg0,)>(1)?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty) => {
        let ($id0, $id1) = $request.args().optional::<($arg0, $arg1)>(2)?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty, $id2:ident: $arg2:ty) => {
        let ($id0, $id1, $id2) = $request.args().optional::<($arg0, $arg1, $arg2)>(3)?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty, $id2:ident: $arg2:ty, $id3:ident: $arg3:ty) => {
        let ($id0, $id1, $id2, $id3) = $request
            .args()
            .optional::<($arg0, $arg1, $arg2, $arg3)>(4)?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty, $id2:ident: $arg2:ty, $id3:ident: $arg3:ty, $id4:ident: $arg4:ty) => {
        let ($id0, $id1, $id2, $id3, $id4) = $request
            .args()
            .optional::<($arg0, $arg1, $arg2, $arg3, $arg4)>(5)?;
    };
}

#[macro_export]
macro_rules! args_match {
    ($request:expr, $id0:ident:$arg0:ty) => {
        let ($id0,) = $request.args().get::<($arg0,)>()?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty) => {
        let ($id0, $id1) = $request.args().get::<($arg0, $arg1)>()?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty, $id2:ident: $arg2:ty) => {
        let ($id0, $id1, $id2) = $request.args().get::<($arg0, $arg1, $arg2)>()?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty, $id2:ident: $arg2:ty, $id3:ident: $arg3:ty) => {
        let ($id0, $id1, $id2, $id3) = $request.args().get::<($arg0, $arg1, $arg2, $arg3)>()?;
    };
    ($request:expr, $id0:ident: $arg0:ty, $id1:ident: $arg1:ty, $id2:ident: $arg2:ty, $id3:ident: $arg3:ty, $id4:ident: $arg4:ty) => {
        let ($id0, $id1, $id2, $id3, $id4) = $request
            .args()
            .get::<($arg0, $arg1, $arg2, $arg3, $arg4)>()?;
    };
}

#[cfg(target_os = "windows")]
pub fn make_base_url(protocol: &str, host: &str) -> String {
    format!("https://{}.{}", protocol, host)
}

#[cfg(target_os = "macos")]
pub fn make_base_url(protocol: &str, host: &str) -> String {
    format!("{}://{}", protocol, host)
}