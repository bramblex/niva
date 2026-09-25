use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use tao::event_loop::ControlFlow;

use super::{NivaApp, NivaEvent, NivaWindowTarget};

/// Run `f` on the main (tao event loop) thread and await its result.
///
/// Safe to call from any thread. NEVER call it from the main thread itself
/// (the main loop never awaits, so this would deadlock by construction).
/// Delivery failures (loop gone) and request timeouts (enforced by the API
/// dispatcher around the whole handler future) both surface as `Err`,
/// never as hangs.
pub async fn run_on_main<T, F>(app: &Arc<NivaApp>, f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&NivaWindowTarget, &mut ControlFlow) -> Result<T> + Send + 'static,
{
    let (tx, rx) = async_channel::bounded::<Result<T>>(1);
    // NivaEvent requires Fn; keep the FnOnce in a shared slot. If the caller's
    // future is dropped before the event starts (for example, API timeout),
    // the guard marks the slot cancelled so the queued event cannot run it.
    let call = Arc::new(PendingMainCall::new(f));
    let _guard = MainCallGuard(call.clone());
    let event_call = call.clone();
    let event = NivaEvent::new(move |target, control_flow| {
        let Some(f) = event_call.take_for_dispatch()? else {
            return Ok(());
        };
        // try_send on an empty bounded(1) never blocks: safe on the loop thread.
        let _ = tx.try_send(f(target, control_flow));
        Ok(())
    });
    app.event_loop_proxy
        .send_event(event)
        .map_err(|_| anyhow!("Event loop is gone"))?;
    rx.recv()
        .await
        .map_err(|_| anyhow!("Main thread dropped the request"))?
}

struct PendingMainCall<F> {
    state: Mutex<PendingMainCallState<F>>,
}

struct PendingMainCallState<F> {
    callback: Option<F>,
    started: bool,
    cancelled: bool,
}

impl<F> PendingMainCall<F> {
    fn new(callback: F) -> Self {
        Self {
            state: Mutex::new(PendingMainCallState {
                callback: Some(callback),
                started: false,
                cancelled: false,
            }),
        }
    }

    fn take_for_dispatch(&self) -> Result<Option<F>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("main-call slot poisoned"))?;
        if state.cancelled || state.started {
            return Ok(None);
        }
        state.started = true;
        Ok(state.callback.take())
    }

    fn cancel_if_not_started(&self) {
        if let Ok(mut state) = self.state.lock()
            && !state.started
        {
            state.cancelled = true;
            state.callback.take();
        }
    }
}

struct MainCallGuard<F>(Arc<PendingMainCall<F>>);

impl<F> Drop for MainCallGuard<F> {
    fn drop(&mut self) {
        self.0.cancel_if_not_started();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn dropped_waiter_prevents_a_queued_main_call_from_starting() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in_callback = calls.clone();
        let pending = Arc::new(PendingMainCall::new(move || {
            calls_in_callback.fetch_add(1, Ordering::SeqCst);
        }));

        drop(MainCallGuard(pending.clone()));
        assert!(pending.take_for_dispatch().unwrap().is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn dropping_waiter_after_dispatch_started_does_not_revoke_callback() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in_callback = calls.clone();
        let pending = Arc::new(PendingMainCall::new(move || {
            calls_in_callback.fetch_add(1, Ordering::SeqCst);
        }));

        let callback = pending.take_for_dispatch().unwrap().unwrap();
        drop(MainCallGuard(pending.clone()));
        callback();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(pending.take_for_dispatch().unwrap().is_none());
    }
}
