use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::Deserialize;
use wry::{
    application::{window::{Window, WindowId}, event_loop::EventLoopWindowTarget},
    webview::WebView,
};

use crate::event::CallbackEvent;

#[derive(Deserialize, Debug)]
pub struct Size(pub f64, pub f64);

#[derive(Deserialize, Debug)]
pub struct Position(pub f64, pub f64);

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MenuItemConfig {
    NativeItem(String),
    MenuItem(String, u16),
    SubMenu(String, Vec<MenuItemConfig>),
}

#[derive(Deserialize, Debug)]
pub struct MenuConfig(pub Vec<MenuItemConfig>);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowOptions {
    // webview config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<(u8, u8, u8, u8)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devtools: Option<bool>,

    // window config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<Size>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximizable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fullscreen: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decorations: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_on_top: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_on_bottom: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_on_all_workspaces: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_protection: Option<bool>,

    // window menu
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu: Option<MenuConfig>,
}

type WindowRef = Arc<Mutex<Window>>;
type WebviewRef = Arc<Mutex<WebView>>;

pub struct WindowManager {
    base_url: String,
    windows: HashMap<WindowId, (WindowRef, WebviewRef)>,
}

impl WindowManager {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            windows: HashMap::new()
        }
    }

    pub fn create(&self, window_options: &WindowOptions, target: &EventLoopWindowTarget<CallbackEvent>) -> Result<WindowId> {
    }

    pub fn close(&self, window_id: WindowId) -> Result<()> {
    }
    
    fn create_webview(&self, window_options: &WindowOptions, window :&Window) -> Result<Webview>{
    }

}
