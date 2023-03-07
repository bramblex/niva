use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Size(pub f64, pub f64);

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Position(pub f64, pub f64);

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum MenuItemConfig {
    NativeItem(String),
    MenuItem(String, u16),
    SubMenu(String, Vec<MenuItemConfig>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MenuConfig(pub Vec<MenuItemConfig>);

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    // project config
    pub name: String,
    pub uuid: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_label: Option<String>,
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

    // runtime config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<usize>,
}
