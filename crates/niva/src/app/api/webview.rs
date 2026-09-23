use anyhow::{Result, anyhow};

use crate::app::NivaApp;
use crate::app::api_manager::ApiManager;
use crate::app::api_manager::ApiRequest;
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("webview.isDevtoolsOpen", is_devtools_open);
    api_manager.register_api("webview.openDevtools", open_devtools);
    api_manager.register_api("webview.closeDevtools", close_devtools);
    api_manager.register_api("webview.baseUrl", base_url);
    api_manager.register_api("webview.baseFileSystemUrl", base_filesystem_url);
    api_manager.register_api("webview.evaluateScript", evaluate_script);
    api_manager.register_api("webview.loadUrl", load_url);
    api_manager.register_api("webview.loadHtml", load_html);
    api_manager.register_api("webview.reload", reload);
    api_manager.register_api("webview.url", url);
    api_manager.register_api("webview.print", print);
    api_manager.register_api("webview.goBack", go_back);
    api_manager.register_api("webview.goForward", go_forward);
    api_manager.register_api("webview.canGoBack", can_go_back);
    api_manager.register_api("webview.canGoForward", can_go_forward);
    api_manager.register_api("webview.cookies", cookies);
    api_manager.register_api("webview.cookiesForUrl", cookies_for_url);
    api_manager.register_api("webview.setCookie", set_cookie);
    api_manager.register_api("webview.deleteCookie", delete_cookie);
    api_manager.register_api("webview.clearAllBrowsingData", clear_all_browsing_data);
}

const MAX_EVALUATE_SCRIPT_BYTES: usize = 64 * 1024;

fn validate_script_size(script: &str) -> Result<()> {
    if script.len() > MAX_EVALUATE_SCRIPT_BYTES {
        return Err(anyhow!(
            "Script exceeds the {} byte limit",
            MAX_EVALUATE_SCRIPT_BYTES
        ));
    }
    Ok(())
}

async fn evaluate_script(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (script,) = request.args().get::<(String,)>()?;
    validate_script_size(&script)?;
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.evaluate_script(&script)?;
        Ok(())
    })
    .await
}

async fn load_url(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (raw_url,) = request.args().get::<(String,)>()?;
    let url = parse_load_url(&raw_url)?;
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.load_url(&url)?;
        Ok(())
    })
    .await
}

fn parse_load_url(raw_url: &str) -> Result<String> {
    let url =
        url::Url::parse(raw_url).map_err(|err| anyhow!("Invalid absolute WebView URL: {err}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(anyhow!(
            "Only HTTP and HTTPS URLs are supported by loadUrl; use loadHtml for inline content"
        ));
    }
    Ok(url.into())
}

async fn load_html(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (html,) = request.args().get::<(String,)>()?;
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.load_html(&html)?;
        Ok(())
    })
    .await
}

async fn reload(app: Arc<NivaApp>, window: Arc<NivaWindow>, _request: ApiRequest) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.reload()?;
        Ok(())
    })
    .await
}

async fn url(app: Arc<NivaApp>, window: Arc<NivaWindow>, _request: ApiRequest) -> Result<String> {
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.webview.url()?)
    })
    .await
}

async fn print(app: Arc<NivaApp>, window: Arc<NivaWindow>, _request: ApiRequest) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.print()?;
        Ok(())
    })
    .await
}

async fn go_back(app: Arc<NivaApp>, window: Arc<NivaWindow>, _request: ApiRequest) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.go_back()?;
        Ok(())
    })
    .await
}

async fn go_forward(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.go_forward()?;
        Ok(())
    })
    .await
}

async fn can_go_back(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<bool> {
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.webview.can_go_back()?)
    })
    .await
}

async fn can_go_forward(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<bool> {
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.webview.can_go_forward()?)
    })
    .await
}

async fn cookies(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Vec<String>> {
    run_on_main(&app, move |_target, _control_flow| {
        Ok(cookies_to_strings(window.webview.cookies()?))
    })
    .await
}

async fn cookies_for_url(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Vec<String>> {
    let (url,) = request.args().get::<(String,)>()?;
    run_on_main(&app, move |_target, _control_flow| {
        Ok(cookies_to_strings(window.webview.cookies_for_url(&url)?))
    })
    .await
}

fn cookies_to_strings(cookies: Vec<wry::cookie::Cookie<'static>>) -> Vec<String> {
    cookies
        .into_iter()
        .map(|cookie| cookie.to_string())
        .collect()
}

async fn set_cookie(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (raw_cookie,) = request.args().get::<(String,)>()?;
    #[cfg(target_os = "android")]
    {
        let _ = (app, window, raw_cookie);
        return Err(anyhow!("Cookie writes are not supported by Wry on Android"));
    }
    #[cfg(not(target_os = "android"))]
    {
        let cookie = parse_cookie(raw_cookie)?;
        run_on_main(&app, move |_target, _control_flow| {
            window.webview.set_cookie(&cookie)?;
            Ok(())
        })
        .await
    }
}

async fn delete_cookie(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (raw_cookie,) = request.args().get::<(String,)>()?;
    #[cfg(target_os = "android")]
    {
        let _ = (app, window, raw_cookie);
        return Err(anyhow!(
            "Cookie deletion is not supported by Wry on Android"
        ));
    }
    #[cfg(not(target_os = "android"))]
    {
        let cookie = parse_cookie(raw_cookie)?;
        run_on_main(&app, move |_target, _control_flow| {
            window.webview.delete_cookie(&cookie)?;
            Ok(())
        })
        .await
    }
}

fn parse_cookie(raw: String) -> Result<wry::cookie::Cookie<'static>> {
    wry::cookie::Cookie::parse(raw)
        .map(wry::cookie::Cookie::into_owned)
        .map_err(|err| anyhow!("Invalid cookie: {err}"))
}

async fn clear_all_browsing_data(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.clear_all_browsing_data()?;
        Ok(())
    })
    .await
}

async fn is_devtools_open(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<bool> {
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.webview.is_devtools_open())
    })
    .await
}

async fn open_devtools(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.open_devtools();
        Ok(())
    })
    .await
}

async fn close_devtools(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.close_devtools();
        Ok(())
    })
    .await
}

async fn base_url(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    let port = app.server_info()?;
    Ok(local_server_base_url(port))
}

fn local_server_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/")
}

async fn base_filesystem_url(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    let port = app.server_info()?;
    Ok(local_file_system_base_url(port, &window.token))
}

fn local_file_system_base_url(port: u16, token: &str) -> String {
    format!("{}__niva_fs/{token}/", local_server_base_url(port))
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_EVALUATE_SCRIPT_BYTES, cookies_to_strings, local_file_system_base_url,
        local_server_base_url, parse_cookie, parse_load_url, validate_script_size,
    };

    #[test]
    fn load_url_accepts_only_absolute_http_or_https_urls() {
        assert_eq!(
            parse_load_url("http://localhost:8080/path").unwrap(),
            "http://localhost:8080/path"
        );
        assert_eq!(
            parse_load_url("https://example.com/path").unwrap(),
            "https://example.com/path"
        );
        assert_eq!(
            parse_load_url("HTTPS://EXAMPLE.COM/path with spaces").unwrap(),
            "https://example.com/path%20with%20spaces"
        );
        assert!(parse_load_url("/relative/path").is_err());
        assert!(parse_load_url("//example.com/path").is_err());
        assert!(parse_load_url("javascript:alert(1)").is_err());
        assert!(parse_load_url("data:text/html,hello").is_err());
        assert!(parse_load_url("file:///tmp/page.html").is_err());
        assert!(parse_load_url("niva://app/index.html").is_err());
        assert!(parse_load_url("ftp://example.com/file").is_err());
    }

    #[test]
    fn evaluate_script_limit_counts_utf8_bytes_and_includes_the_boundary() {
        assert!(validate_script_size(&"x".repeat(MAX_EVALUATE_SCRIPT_BYTES)).is_ok());
        assert!(validate_script_size(&"x".repeat(MAX_EVALUATE_SCRIPT_BYTES + 1)).is_err());

        let within_limit = "界".repeat(MAX_EVALUATE_SCRIPT_BYTES / "界".len());
        assert_eq!(within_limit.len(), MAX_EVALUATE_SCRIPT_BYTES - 1);
        assert!(validate_script_size(&within_limit).is_ok());
        assert!(validate_script_size(&format!("{within_limit}界")).is_err());
    }

    #[test]
    fn cookie_arguments_are_parsed_and_formatted_as_set_cookie_values() {
        let cookie =
            parse_cookie("session=hello; Path=/; HttpOnly; SameSite=Lax".to_owned()).unwrap();
        assert_eq!(cookie.name(), "session");
        assert_eq!(cookie.value(), "hello");
        assert_eq!(
            cookies_to_strings(vec![cookie]),
            vec!["session=hello; HttpOnly; SameSite=Lax; Path=/"]
        );

        assert!(parse_cookie("missing-separator".to_owned()).is_err());
    }

    #[test]
    fn local_webview_urls_include_the_server_port_and_window_file_token() {
        assert_eq!(local_server_base_url(43_210), "http://127.0.0.1:43210/");
        assert_eq!(
            local_file_system_base_url(43_210, "0123456789abcdef0123456789abcdef"),
            "http://127.0.0.1:43210/__niva_fs/0123456789abcdef0123456789abcdef/"
        );
    }
}
