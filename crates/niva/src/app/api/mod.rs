use super::api_manager::ApiManager;

mod clipboard;
mod dialog;
mod extra;
mod fs;
mod http;
pub(crate) mod module;
mod monitor;
pub(crate) mod os;
mod process;
mod resource;
mod shortcut;
mod socket;
mod tray;
mod webview;
mod window;
mod window_extra;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    dialog::register_api_instances(api_manager);
    window::register_api_instances(api_manager);
    fs::register_api_instances(api_manager);
    http::register_apis(api_manager);
    os::register_apis(api_manager);
    process::register_apis(api_manager);
    socket::register_api_instances(api_manager);
    webview::register_apis(api_manager);
    resource::register_apis(api_manager);
    clipboard::register_api_instances(api_manager);
    shortcut::register_api_instances(api_manager);
    tray::register_api_instances(api_manager);
    monitor::register_api_instances(api_manager);
    module::register_apis(api_manager);
    extra::register_api_instances(api_manager);
    window_extra::register_api_instances(api_manager);
}

pub(crate) fn cancel_process_call(window_id: u8, connection_id: u64, call_id: u64) {
    process::cancel_bridge_child(window_id, connection_id, call_id);
}

pub(crate) fn cancel_process_connection(window_id: u8, connection_id: u64) {
    process::cancel_bridge_session(window_id, connection_id);
}

pub(crate) fn cancel_process_window(window_id: u8) {
    process::cancel_window_children(window_id);
}

pub(crate) fn cancel_sync_process_call(window_id: u8, session_id: &str, call_id: u64) {
    process::cancel_sync_child(window_id, session_id, call_id);
}

pub(crate) fn cancel_sync_process_session(window_id: u8, session_id: &str) {
    process::cancel_sync_session(window_id, session_id);
}

pub(crate) fn cancel_sync_file_session(window_id: u8, session_id: &str) {
    fs::cancel_sync_fd_session(window_id, session_id);
}

pub(crate) fn cancel_sync_file_window(window_id: u8) {
    fs::cancel_sync_fd_window(window_id);
}

pub(crate) fn cancel_ipc_process_call(
    window_id: u8,
    source_origin: &str,
    session_id: &str,
    frame_id: u64,
    generation: u64,
    call_id: u64,
) {
    process::cancel_ipc_child(
        window_id,
        source_origin,
        session_id,
        frame_id,
        generation,
        call_id,
    );
}

pub(crate) fn cancel_ipc_process_session(
    window_id: u8,
    source_origin: &str,
    session_id: &str,
    frame_id: u64,
    generation: u64,
) {
    process::cancel_ipc_session(window_id, source_origin, session_id, frame_id, generation);
}

pub(crate) fn cancel_ipc_process_frame(window_id: u8, frame_id: u64, generation: u64) {
    process::cancel_ipc_frame(window_id, frame_id, generation);
}

pub(crate) fn cancel_ipc_process_window(window_id: u8) {
    process::cancel_ipc_window(window_id);
}
