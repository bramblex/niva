//! Thread-bound objects never enter the Send/Sync application graph.
use anyhow::{Result, anyhow, ensure};
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    thread::{self, ThreadId},
};

thread_local! {
    static OBJECTS: RefCell<HashMap<u64, Box<dyn Any>>> = RefCell::new(HashMap::new());
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub(super) struct UiHandle<T: 'static> {
    id: u64,
    owner: ThreadId,
    marker: PhantomData<fn() -> T>,
}

impl<T: 'static> UiHandle<T> {
    pub fn new(value: T) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        OBJECTS.with(|objects| objects.borrow_mut().insert(id, Box::new(Rc::new(value))));
        Self {
            id,
            owner: thread::current().id(),
            marker: PhantomData,
        }
    }

    pub fn get(&self) -> Result<Rc<T>> {
        ensure!(
            thread::current().id() == self.owner,
            "UI object accessed outside its owner thread"
        );
        OBJECTS.with(|objects| {
            objects
                .borrow()
                .get(&self.id)
                .and_then(|value| value.downcast_ref::<Rc<T>>())
                .cloned()
                .ok_or_else(|| anyhow!("window UI has been closed"))
        })
    }

    pub fn close(&self) -> Result<()> {
        ensure!(
            thread::current().id() == self.owner,
            "UI object closed outside its owner thread"
        );
        // Drop outside the RefCell borrow: native destruction can re-enter callbacks.
        let value = OBJECTS.with(|objects| objects.borrow_mut().remove(&self.id));
        drop(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_handles_cannot_access_or_destroy_ui() {
        struct Probe(Rc<RefCell<Option<ThreadId>>>);
        impl Drop for Probe {
            fn drop(&mut self) {
                *self.0.borrow_mut() = Some(thread::current().id());
            }
        }
        let dropped = Rc::new(RefCell::new(None));
        let handle = std::sync::Arc::new(UiHandle::new(Probe(dropped.clone())));
        let worker = handle.clone();
        thread::spawn(move || {
            assert!(worker.get().is_err());
            assert!(worker.close().is_err());
        })
        .join()
        .unwrap();
        assert!(dropped.borrow().is_none());
        handle.close().unwrap();
        assert_eq!(*dropped.borrow(), Some(thread::current().id()));
        assert!(handle.get().is_err());
        thread::spawn(move || drop(handle)).join().unwrap();
    }
}
