pub type SharePtr<T> = Arc<>;

pub fn mk_shared<T>(t: T) -> SharePtr<T> {
    Arc::new(t)
}

pub type MutSharePtr<T> = Arc<Mutex<T>>;

pub fn mk_mut_shared<T>(t: T) -> MutSharePtr<T> {
    Arc::new(Mutex::new(t))
}

pub type FuncPtr<F> = Pin<Box<F>>;

pub fn make_func_ptr<F>(f: F) -> FuncPtr<F> {
    Box::pin(f)
}