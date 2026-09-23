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

async fn evaluate_script(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (script,) = request.args().get::<(String,)>()?;
    if script.len() > MAX_EVALUATE_SCRIPT_BYTES {
        return Err(anyhow!(
            "Script exceeds the {} byte limit",
            MAX_EVALUATE_SCRIPT_BYTES
        ));
    }
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
        Ok(window
            .webview
            .cookies()?
            .into_iter()
            .map(|cookie| cookie.to_string())
            .collect())
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
        Ok(window
            .webview
            .cookies_for_url(&url)?
            .into_iter()
            .map(|cookie| cookie.to_string())
            .collect())
    })
    .await
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
    Ok(format!("http://127.0.0.1:{port}/"))
}

async fn base_filesystem_url(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    let port = app.server_info()?;
    Ok(format!(
        "http://127.0.0.1:{port}/__niva_fs/{}/",
        window.token
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_load_url;

    #[test]
    fn load_url_requires_an_absolute_http_or_https_url() {
        assert_eq!(
            parse_load_url("http://localhost:8080/path").unwrap(),
            "http://localhost:8080/path"
        );
        assert_eq!(
            parse_load_url("https://example.com/path").unwrap(),
            "https://example.com/path"
        );
        assert!(parse_load_url("/relative/path").is_err());
        assert!(parse_load_url("javascript:alert(1)").is_err());
        assert!(parse_load_url("data:text/html,hello").is_err());
    }
}
