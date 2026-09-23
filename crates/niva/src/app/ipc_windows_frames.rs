use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Result, anyhow};
use webview2_com::{
    FrameChildFrameCreatedEventHandler, FrameCreatedEventHandler, FrameDestroyedEventHandler,
    FrameNavigationStartingEventHandler, FrameWebMessageReceivedEventHandler,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_4, ICoreWebView2Frame, ICoreWebView2Frame2, ICoreWebView2Frame7,
    },
    take_pwstr,
};
use windows::core::{Interface, PWSTR};
use wry::{WebView, WebViewExtWindows};

use super::NivaApp;

struct FrameState {
    frame: ICoreWebView2Frame2,
    navigation_generation: Arc<AtomicU64>,
}

thread_local! {
    // WebView2 COM objects are apartment-bound. Keep them on the UI thread and
    // pass only a key through the async IPC task.
    static FRAMES: RefCell<HashMap<u64, FrameState>> = RefCell::new(HashMap::new());
}

static NEXT_FRAME_KEY: AtomicU64 = AtomicU64::new(1);

/// Install WebView2 frame IPC handlers. Call on the WebView's UI thread.
pub fn install(webview: &WebView, app: Arc<NivaApp>, window_id: u8) -> Result<()> {
    let webview = webview.webview();
    let webview4: ICoreWebView2_4 = webview.cast()?;
    let weak_app = Arc::downgrade(&app);

    let handler = FrameCreatedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else {
            return Ok(());
        };
        let frame = unsafe { args.Frame()? };
        if let Err(error) = attach_frame(&frame, weak_app.clone(), window_id) {
            eprintln!("[niva] unable to attach iframe IPC handler: {error}");
        }
        Ok(())
    }));

    let mut token = 0;
    unsafe { webview4.add_FrameCreated(&handler, &mut token)? };
    Ok(())
}

fn attach_frame(frame: &ICoreWebView2Frame, app: Weak<NivaApp>, window_id: u8) -> Result<()> {
    let frame2: ICoreWebView2Frame2 = frame.cast()?;
    let frame_key = NEXT_FRAME_KEY.fetch_add(1, Ordering::Relaxed);
    let navigation_generation = Arc::new(AtomicU64::new(0));

    let navigation_counter = navigation_generation.clone();
    let navigation_handler = FrameNavigationStartingEventHandler::create(Box::new(move |_, _| {
        navigation_counter.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }));
    let mut navigation_token = 0;
    unsafe { frame2.add_NavigationStarting(&navigation_handler, &mut navigation_token)? };

    let message_app = app.clone();
    let message_generation = navigation_generation.clone();
    let message_handler = FrameWebMessageReceivedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else {
            return Ok(());
        };

        let mut source = PWSTR::null();
        unsafe { args.Source(&mut source)? };
        let source_url = take_pwstr(source);

        let mut message = PWSTR::null();
        unsafe { args.TryGetWebMessageAsString(&mut message)? };
        let body = take_pwstr(message);

        let generation = message_generation.load(Ordering::Acquire);
        let navigation_generation = message_generation.clone();
        let Some(app) = message_app.upgrade() else {
            return Ok(());
        };

        smol::spawn(async move {
            let response = match app.api().ipc_call(window_id, &source_url, &body).await {
                Ok(response) => response,
                Err(error) => {
                    eprintln!("[niva] rejected iframe IPC request: {error}");
                    crate::app::api_manager::ApiManager::ipc_error_response(
                        &body,
                        &error.to_string(),
                    )
                }
            };

            if let Err(error) = crate::app::main_exec::run_on_main(&app, move |_, _| {
                if navigation_generation.load(Ordering::Acquire) != generation {
                    return Ok(());
                }
                let frame = FRAMES.with(|frames| {
                    let frames = frames.try_borrow().ok()?;
                    let state = frames.get(&frame_key)?;
                    Arc::ptr_eq(&state.navigation_generation, &navigation_generation)
                        .then(|| state.frame.clone())
                });

                let Some(frame) = frame else {
                    // The iframe navigated or was destroyed while ipc_call ran.
                    return Ok(());
                };

                unsafe {
                    frame.PostWebMessageAsString(&windows::core::HSTRING::from(response))?;
                }
                Ok(())
            })
            .await
            {
                eprintln!("[niva] unable to reply to iframe IPC request: {error}");
            }
        })
        .detach();

        Ok(())
    }));
    let mut message_token = 0;
    unsafe { frame2.add_WebMessageReceived(&message_handler, &mut message_token)? };

    let destroyed_handler = FrameDestroyedEventHandler::create(Box::new(move |_, _| {
        FRAMES.with(|frames| {
            if let Ok(mut frames) = frames.try_borrow_mut() {
                frames.remove(&frame_key);
            }
        });
        Ok(())
    }));
    let mut destroyed_token = 0;
    unsafe { frame.add_Destroyed(&destroyed_handler, &mut destroyed_token)? };

    FRAMES.with(|frames| -> Result<()> {
        frames
            .try_borrow_mut()
            .map_err(|_| anyhow!("iframe registry is busy"))?
            .insert(
                frame_key,
                FrameState {
                    frame: frame2.clone(),
                    navigation_generation,
                },
            );
        Ok(())
    })?;

    // Listen recursively where the installed WebView2 Runtime supports child
    // frame creation events. Older runtimes still support direct iframe IPC.
    if let Ok(frame7) = frame.cast::<ICoreWebView2Frame7>() {
        let child_app = app;
        let child_handler = FrameChildFrameCreatedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else {
                return Ok(());
            };
            let child = unsafe { args.Frame()? };
            if let Err(error) = attach_frame(&child, child_app.clone(), window_id) {
                eprintln!("[niva] unable to attach nested iframe IPC handler: {error}");
            }
            Ok(())
        }));
        let mut child_token = 0;
        if let Err(error) = unsafe { frame7.add_FrameCreated(&child_handler, &mut child_token) } {
            eprintln!("[niva] unable to observe nested iframes: {error}");
        }
    }

    Ok(())
}
