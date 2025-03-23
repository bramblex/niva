// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::str::FromStr;

use anyhow::Result;
use winit::{
    application::ApplicationHandler,
    event_loop::{EventLoop, EventLoopProxy},
    window::Window,
};
use wry::{
    self, WebViewBuilder,
    http::{Request, Response, header::CONTENT_TYPE},
};

use tiny_http::{Header, HeaderField, Response as TinyResponse, Server};
use ureq::config::Config;
use ureq::tls::{TlsConfig, TlsProvider};

#[derive(Default)]
struct App {
    window: Option<Window>,
    webview: Option<wry::WebView>,
}

impl App {
    fn start_server(&mut self) {
        std::thread::spawn(move || {
            let server = Server::http("127.0.0.1:8080").unwrap();

            for request in server.incoming_requests() {
                let response = TinyResponse::from_string(
                    "<html>
                    <body>
                        <h1>Hello, World!</h1>
                        <iframe src='./index.html' width='100%' height='100%'></iframe>
                    </body>
                    </html>
                    ",
                )
                .with_header(Header::from_str("Content-Type: text/html; charset=utf-8").unwrap());
                request.respond(response).unwrap();
            }
        });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        let webview = WebViewBuilder::new()
            .with_url("http://127.0.0.1:8080")
            .build(&window)
            .unwrap();

        self.window = Some(window);
        self.webview = Some(webview);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
    }
}

struct NivaOptions {
    // 基础选项
    uuid: String,
    name: String,
    version: String,

    // meta 信息
    description: Option<String>,
    icon: Option<String>,
    copyright: Option<String>,
    company: Option<String>,
}

struct ResourceOptions {
}

fn main() -> Result<()> {
    // let mut config = Config::builder()
    //     .tls_config(
    //         TlsConfig::builder()
    //             // requires the native-tls feature
    //             .provider(TlsProvider::NativeTls)
    //             .build(),
    //     )
    //     .build();

    // let agent = config.new_agent();

    // // agent.get("https://www.google.com/").call().unwrap();

    // let result = agent.get("https://www.github.com").call().unwrap();
    // println!("{:?}", result.status());

    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    app.start_server();
    event_loop.run_app(&mut app)?;
    Ok(())
}
