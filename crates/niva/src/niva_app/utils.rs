use anyhow::{anyhow, Result};
use std::sync::{Arc, Mutex};

pub type ArcMut<T> = Arc<Mutex<T>>;

pub fn arc<T>(t: T) -> Arc<T> {
    Arc::new(t)
}

pub fn arc_mut<T>(t: T) -> ArcMut<T> {
    Arc::new(Mutex::new(t))
}

pub struct Counter {
    next_id: u32,
}

impl Counter {
    pub fn new(start: u32) -> Self {
        Self { next_id: start }
    }

    pub fn next(&mut self) -> u32 {
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
