use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Size {
    pub fn to_dpi_size(&self) -> wry::application::dpi::Size {
        use wry::application::dpi;
        return dpi::Size::Logical(dpi::LogicalSize {
            width: self.width,
            height: self.height,
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MenuConfig {
    pub label: String,
    pub accelerator: Option<String>,
    pub click: Option<String>,
    pub submenu: Option<Vec<MenuConfig>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    // project config
    pub name: String,
    pub version: Option<String>,
    pub entry: Option<String>, // default is "index.html"
    pub title: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,

    // webview config
    pub devtools: Option<bool>, // default is false

    // window config
    pub theme: Option<String>, // default is "light" | "dark"
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,

    pub position: Option<Size>,

    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
    pub closable: Option<bool>,

    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub visible: Option<bool>,
    pub transparent: Option<bool>,
    pub background_color: Option<(u8, u8, u8, u8)>, // default is (255, 255, 255, 255)
    pub decorations: Option<bool>,

    pub always_on_top: Option<bool>,
    pub always_on_bottom: Option<bool>,
    pub visible_on_all_workspaces: Option<bool>,

    pub focused: Option<bool>,
    pub content_protection: Option<bool>,

    // window menu
    pub menu: Option<Vec<MenuConfig>>,
}

pub fn get_config(path: &std::path::Path) -> Config {
    let content = std::fs::read_to_string(path);
    if content.is_err() {
        panic!("[Error] Read config file error");
    }
    let config = serde_json::from_str(&content.unwrap());
    if config.is_err() {
        panic!("[Error] Parse config file error");
    }
    return config.unwrap();
}
