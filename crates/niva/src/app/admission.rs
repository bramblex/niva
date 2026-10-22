//! Admission happens before allocating task state, not after dispatch.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
pub(crate) struct Admission(Arc<AtomicUsize>);
pub(crate) struct Permit(Arc<AtomicUsize>);
impl Admission {
    pub fn acquire(&self, limit: usize) -> Option<Permit> {
        self.0
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < limit).then_some(count + 1)
            })
            .ok()?;
        Some(Permit(self.0.clone()))
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permits_bound_live_work_and_release_on_drop() {
        let gate = Admission::default();
        let first = gate.acquire(1).unwrap();
        assert!(gate.acquire(1).is_none());
        assert!(gate.acquire(0).is_none());
        std::thread::spawn(move || drop(first)).join().unwrap();
        assert!(gate.acquire(1).is_some());
    }
}
