use std::{
    cell::RefCell,
    collections::HashMap,
    ptr,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Result, anyhow};
use block2::{DynBlock, RcBlock};
use objc2::{
    DeclaredClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, ProtocolObject},
};
use objc2_foundation::{NSObjectProtocol, NSString, ns_string};
use objc2_web_kit::{
    WKContentWorld, WKScriptMessage, WKScriptMessageHandlerWithReply, WKUserContentController,
};
use wry::{WebView, WebViewExtMacOS};

use super::{NivaApp, NivaEvent};

const REPLY_HANDLER_NAME: &str = "nivaReply";

type ReplyBlock = dyn Fn(*mut AnyObject, *mut NSString);

// WKScriptMessageHandlerWithReply is main-thread-only, and WebKit's reply
// block must not be sent to the API executor. Keep copied blocks on the main
// thread and send only a numeric key through NivaEvent.
thread_local! {
    static PENDING_REPLIES: RefCell<HashMap<u64, RcBlock<ReplyBlock>>> =
        RefCell::new(HashMap::new());
}

static NEXT_REPLY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
struct ReplyHandlerIvars {
    app: Weak<NivaApp>,
    window_id: u8,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ReplyHandlerIvars]
    struct ReplyHandler;

    unsafe impl NSObjectProtocol for ReplyHandler {}

    unsafe impl WKScriptMessageHandlerWithReply for ReplyHandler {
        #[unsafe(method(userContentController:didReceiveScriptMessage:replyHandler:))]
        unsafe fn did_receive(
            &self,
            _controller: &WKUserContentController,
            message: &WKScriptMessage,
            reply_handler: &DynBlock<ReplyBlock>,
        ) {
            let result = self.handle_message(message, reply_handler);
            if let Err(error) = result {
                reject_reply(reply_handler, &error.to_string());
            }
        }
    }
);

impl ReplyHandler {
    fn handle_message(
        &self,
        message: &WKScriptMessage,
        reply_handler: &DynBlock<ReplyBlock>,
    ) -> Result<()> {
        let body = unsafe { message.body() }
            .downcast::<NSString>()
            .map_err(|_| anyhow!("nivaReply expects a JSON string message body"))?
            .to_string();

        // Use the sending frame's URL, not the current WKWebView URL. A
        // navigation can occur while an asynchronous request is in flight;
        // ipc_message owns all source validation.
        let frame_info = unsafe { message.frameInfo() };
        let is_main_frame = unsafe { frame_info.isMainFrame() };
        let request = unsafe { frame_info.request() };
        let source_url = request
            .URL()
            .and_then(|url| url.absoluteString())
            .map(|url| url.to_string())
            .ok_or_else(|| anyhow!("nivaReply message has no source frame URL"))?;

        let app = self
            .ivars()
            .app
            .upgrade()
            .ok_or_else(|| anyhow!("Niva app is shutting down"))?;
        let window_id = self.ivars().window_id;
        let event_loop_proxy = app.event_loop_proxy.clone();
        let reply_id = NEXT_REPLY_ID.fetch_add(1, Ordering::Relaxed);

        let inserted = PENDING_REPLIES.with(|pending| {
            pending
                .try_borrow_mut()
                .map(|mut pending| pending.insert(reply_id, reply_handler.copy()))
                .map_err(|_| anyhow!("nivaReply callback registry is busy"))
        })?;
        debug_assert!(inserted.is_none());

        // smol's global executor allows WebKit's main thread to return
        // immediately. The callback itself stays in PENDING_REPLIES until a
        // NivaEvent removes and invokes it on the main event loop.
        let spawn_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            smol::spawn(async move {
                let result = app
                    .api()
                    .ipc_message(
                        window_id,
                        super::api_manager::IpcFrameSource {
                            source_url: source_url.clone(),
                            frame_id: 0,
                            generation: 0,
                            is_main_frame,
                        },
                        &body,
                    )
                    .await
                    .map_err(|error| error.to_string());

                let result = result.clone();
                let event = NivaEvent::new(move |_, _| {
                    let reply = PENDING_REPLIES.with(|pending| {
                        pending
                            .try_borrow_mut()
                            .ok()
                            .and_then(|mut pending| pending.remove(&reply_id))
                    });

                    if let Some(reply) = reply {
                        fulfill_or_reject(&reply, result.clone());
                    }
                    Ok(())
                });

                if event_loop_proxy.send_event(event).is_err() {
                    // The event loop is already closed, so its main-thread
                    // callback cannot run. The block remains thread-local and
                    // is released when the main thread exits.
                    crate::niva_log!(
                        crate::app::logging::Level::Warn,
                        "[niva] unable to reply to nivaReply: event loop is closed"
                    );
                }
            })
        }));

        match spawn_result {
            Ok(task) => {
                task.detach();
                Ok(())
            }
            Err(_) => {
                PENDING_REPLIES.with(|pending| {
                    if let Ok(mut pending) = pending.try_borrow_mut() {
                        pending.remove(&reply_id);
                    }
                });
                Err(anyhow!("unable to schedule nivaReply request"))
            }
        }
    }
}

/// Install the macOS asynchronous, non-streaming JSON IPC handler.
///
/// `WKScriptMessageHandlerWithReply` is available on macOS 11 and newer. The
/// user-content controller retains its handler, which keeps no strong app
/// reference to avoid a WebView/App reference cycle.
pub fn install(webview: &WebView, app: Arc<NivaApp>, window_id: u8) -> Result<()> {
    let mtm =
        MainThreadMarker::new().ok_or_else(|| anyhow!("install must run on the main thread"))?;
    let allocated = mtm.alloc::<ReplyHandler>().set_ivars(ReplyHandlerIvars {
        app: Arc::downgrade(&app),
        window_id,
    });
    let handler: Retained<ReplyHandler> = unsafe { msg_send![super(allocated), init] };
    let protocol_handler = ProtocolObject::from_ref(&*handler);
    let content_world = unsafe { WKContentWorld::pageWorld(mtm) };

    unsafe {
        webview
            .manager()
            .addScriptMessageHandlerWithReply_contentWorld_name(
                protocol_handler,
                &content_world,
                ns_string!(REPLY_HANDLER_NAME),
            );
    }

    // WKUserContentController retains the protocol object after registration.
    drop(handler);
    Ok(())
}

fn fulfill_or_reject(
    reply_handler: &RcBlock<ReplyBlock>,
    result: std::result::Result<String, String>,
) {
    match result {
        Ok(response) => {
            let response = NSString::from_str(&response);
            reply_handler.call((
                Retained::as_ptr(&response) as *mut AnyObject,
                ptr::null_mut(),
            ));
        }
        Err(error) => {
            let error = NSString::from_str(&error);
            reply_handler.call((ptr::null_mut(), Retained::as_ptr(&error) as *mut NSString));
        }
    }
}

fn reject_reply(reply_handler: &DynBlock<ReplyBlock>, error: &str) {
    let error = NSString::from_str(error);
    reply_handler.call((ptr::null_mut(), Retained::as_ptr(&error) as *mut NSString));
}
