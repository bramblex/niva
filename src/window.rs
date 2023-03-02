use super::config::Config;

include!(concat!(env!("OUT_DIR"), "/preload.rs"));

pub fn open_main_window(entry_url: &String, config: &Config) -> ! {
    use wry::{
        application::{
            event::{Event, StartCause, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            menu::MenuBar,
            menu::MenuItem,
            window::WindowBuilder,
        },
        webview::WebViewBuilder,
    };

    let event_loop = EventLoop::new();

    // window config
    let mut window_builder = WindowBuilder::new();

    if config.title.is_some() {
        window_builder = window_builder.with_title(config.title.as_ref().unwrap());
    }

    // if config.icon.is_some() {
    //     window_builder.with_window_icon(Some(config.icon.as_ref().unwrap()));
    // }

    if config.size.is_some() {
        let size = config.size.as_ref().unwrap();
        window_builder = window_builder.with_inner_size(size.to_dpi_size());
    }

    if config.min_size.is_some() {
        let min_size = config.min_size.as_ref().unwrap();
        window_builder = window_builder.with_min_inner_size(min_size.to_dpi_size());
    }

    if config.max_size.is_some() {
        let max_size = config.max_size.as_ref().unwrap();
        window_builder = window_builder.with_max_inner_size(max_size.to_dpi_size());
    }

    if config.resizable.is_some() {
        window_builder = window_builder.with_resizable(config.resizable.unwrap());
    }

    if config.always_on_top.is_some() {
        window_builder = window_builder.with_always_on_top(config.always_on_top.unwrap());
    }

    if config.always_on_bottom.is_some() {
        window_builder = window_builder.with_always_on_bottom(config.always_on_bottom.unwrap());
    }

    let mut menu = MenuBar::new();
    let mut file_menu = MenuBar::new();
    file_menu.add_native_item(MenuItem::Cut);
    file_menu.add_native_item(MenuItem::Copy);
    file_menu.add_native_item(MenuItem::Paste);
    file_menu.add_native_item(MenuItem::Quit);
    menu.add_submenu("File", true, file_menu);
    window_builder = window_builder.with_menu(menu);

    let window = window_builder.build(&event_loop).unwrap();

    // webview config
    let mut webview_builder = WebViewBuilder::new(window).unwrap();

    if config.background_color.is_some() {
        webview_builder = webview_builder
            .with_background_color(config.background_color.as_ref().unwrap().clone());
    }

    if config.devtools.is_some() {
        webview_builder = webview_builder.with_devtools(config.devtools.unwrap());
    }

    webview_builder = webview_builder.with_initialization_script(PRELOAD_JS);
    webview_builder = webview_builder.with_clipboard(true);

    let _webview = webview_builder
        .with_url(entry_url)
        .unwrap()
        .build()
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
