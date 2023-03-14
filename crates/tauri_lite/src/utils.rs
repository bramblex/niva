use std::{sync::{Arc, Mutex}, pin::Pin};

pub type SharePtr<T> = Arc<T>;

#[inline]
pub fn mk_shared<T>(t: T) -> SharePtr<T> {
    Arc::new(t)
}

pub type MutSharePtr<T> = Arc<Mutex<T>>;

#[inline]
pub fn mk_mut_shared<T>(t: T) -> MutSharePtr<T> {
    Arc::new(Mutex::new(t))
}

pub type FuncPtr<F> = Pin<Box<F>>;

#[inline]
pub fn make_func_ptr<F>(f: F) -> FuncPtr<F> {
    Box::pin(f)
}

pub struct Counter {
    next_id: u32,
}

impl Counter {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn next(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}