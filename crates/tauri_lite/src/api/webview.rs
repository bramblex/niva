use serde::Deserialize;
use serde_json::json;
use wry::webview::WebView;
use super::{ApiResponse, ApiRequest};


pub fn call(webview: &WebView, request: ApiRequest) -> ApiResponse {
    match request.method.as_str() {
        "isDevtoolsOpen" => is_devtools_open(webview, request),
        "openDevtools" => open_devtools(webview, request),
        "closeDevtools" => close_devtools(webview, request),
        "setBackgroundColor" => set_background_color(webview, request),
        _ => ApiResponse::err(request.callback_id, "method not found"),
    }
}

fn is_devtools_open(webview: &WebView, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, webview.is_devtools_open())
}

fn open_devtools(webview: &WebView, request: ApiRequest) -> ApiResponse {
    webview.open_devtools();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn close_devtools(webview: &WebView, request: ApiRequest) -> ApiResponse {
    webview.close_devtools();
    ApiResponse::ok(request.callback_id, json!({}))
}

#[derive(Debug, Deserialize)]
struct SetBackgroundColorOptions {
    color: (u8, u8, u8, u8),
}

fn set_background_color(webview: &WebView, request: ApiRequest) -> ApiResponse {
    let options = serde_json::from_value::<SetBackgroundColorOptions>(request.data);
    if options.is_err() {
        return ApiResponse::err(
            request.callback_id,
            "Invalid Request.",
        );
    }
    let result = webview.set_background_color(options.unwrap().color);
    if result.is_err() {
        return ApiResponse::err(
            request.callback_id,
            "Cannot set background color.",
        );
    };
    ApiResponse::ok(request.callback_id, json!({}))
}