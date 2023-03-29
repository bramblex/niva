use std::sync::{Arc, Mutex};

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
            .map_err(|_| anyhow::anyhow!("Failed to lock {}.", stringify!($value)))?
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