use serde::Deserialize;
use tao::dpi::{LogicalPosition, LogicalSize};

use crate::app::common::menu_options::MenuOptions;

pub type NivaSize = LogicalSize<f64>;
pub type NivaPosition = LogicalPosition<f64>;

#[cfg(target_os = "macos")]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MacWindowExtraOptions {
    pub parent_window: Option<u8>,
    pub movable_by_window_background: Option<bool>,
    pub title_bar_transparent: Option<bool>,
    pub title_bar_hidden: Option<bool>,
    pub title_bar_buttons_hidden: Option<bool>,
    pub title_hidden: Option<bool>,
    pub full_size_content_view: Option<bool>,
    pub resize_increments: Option<NivaSize>,
    pub disallow_hi_dpi: Option<bool>,
    pub has_shadow: Option<bool>,
    pub automatic_window_tabbing: Option<bool>,
    pub tabbing_identifier: Option<String>,
}

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WinWindowExtraOptions {
    pub parent_window: Option<u8>,
    pub owner_window: Option<u8>,
    pub taskbar_icon: Option<String>,
    pub skip_taskbar: Option<bool>,
    pub undecorated_shadow: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct NivaWindowOptions {
    #[serde(default)]
    pub entry: String,
    pub preload: Option<String>,
    #[serde(default)]
    pub env: serde_json::Value,
    pub custom_close_request: Option<bool>, // block native close request
    pub devtools: Option<bool>,

    pub title: Option<String>,
    pub icon: Option<String>,
    pub theme: Option<String>,
    pub size: Option<NivaSize>,
    pub min_size: Option<NivaSize>,
    pub max_size: Option<NivaSize>,

    pub position: Option<NivaPosition>,

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

    // macos extra
    #[cfg(target_os = "macos")]
    #[serde(flatten)]
    pub macos_extra: Option<MacWindowExtraOptions>,

    // windows extra
    #[cfg(target_os = "windows")]
    #[serde(flatten)]
    pub windows_extra: Option<WinWindowExtraOptions>,

    // menu options
    pub menu: Option<Vec<MenuOptions>>,
}
