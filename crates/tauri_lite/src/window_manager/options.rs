use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Size(pub f64, pub f64);

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Position(pub f64, pub f64);

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MenuItemOptions {
    NativeItem(String),
    MenuItem(String, u16),
    SubMenu(String, Vec<MenuItemOptions>),
}

#[derive(Deserialize, Debug)]
pub struct MenuOptions(pub Vec<MenuItemOptions>);

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowOptions {
    // webview config
    pub entry: Option<String>,
    pub background_color: Option<(u8, u8, u8, u8)>,
    pub devtools: Option<bool>,

    // window config
    pub title: Option<String>,
    pub theme: Option<String>,
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,

    pub position: Option<Position>,

    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
    pub closable: Option<bool>,

    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub visible: Option<bool>,
    pub transparent: Option<bool>,
    pub decorations: Option<bool>,

    pub always_on_top: Option<bool>,
    pub always_on_bottom: Option<bool>,
    pub visible_on_all_workspaces: Option<bool>,

    pub focused: Option<bool>,
    pub content_protection: Option<bool>,

    // window menu
    pub menu: Option<MenuOptions>,
}