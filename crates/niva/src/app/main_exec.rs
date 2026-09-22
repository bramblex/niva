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
    // NivaEvent requires Fn; stash the FnOnce behind a mutex (taken once).
    let slot = Mutex::new(Some(f));
    let event = NivaEvent::new(move |target, control_flow| {
        let f = slot
            .lock()
            .map_err(|_| anyhow!("main-call slot poisoned"))?
            .take()
            .ok_or(anyhow!("main-call consumed twice"))?;
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
