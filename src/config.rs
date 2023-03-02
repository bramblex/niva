use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Size {
    pub width: f64,
    pub height: f64,
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
pub struct Config {
    // webview entry
    pub entry: String,

    // webview config
    pub background_color: Option<(u8, u8, u8, u8)>,
    pub devtools: Option<bool>,

    // window config
    pub title: Option<String>,
    pub icon: Option<String>,
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub resizable: Option<bool>,
    pub always_on_top: Option<bool>,
    pub always_on_bottom: Option<bool>,
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
