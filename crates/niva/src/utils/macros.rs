#[macro_export]
macro_rules! lock {
    ($value:ident, $body:block) => {
        lock!($value, $value, $body)
    };
    ($value:expr, $name:ident, $body:block) => {
        if let Ok(mut $name) = $value.lock() {
            $body
        }
    };
}

#[macro_export]
macro_rules! with_manager {
    ($app:expr, $($name:ident:$type:ty,)+ $body:block) => {
        {
            if let ($(Some($name),)*) = ($($app.get_manager::<$type>(),)*) {
                if let ($(Ok(mut $name),)*) = ($($name.lock(),)*) {
                    $(let $name = $name.as_any().downcast_mut::<$type>();)*
                    if let ($(Some($name),)*) = ($($name,)*) {
                        $body
                    }
                }
            }
        }
    };
}
