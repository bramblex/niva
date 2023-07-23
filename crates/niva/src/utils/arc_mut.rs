use std::sync::{Arc, Mutex};

pub type ArcMut<T> = Arc<Mutex<T>>;

#[macro_export]
macro_rules! with_lock {
    ($($name:ident = $value:expr,)+ $body:block) => {
        if let ($(Ok(mut $name),)+) = ($($value.lock(),)+) {
            $body
        }
    };
}
