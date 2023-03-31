use serde::Deserialize;
use tao::dpi::{LogicalSize, LogicalPosition};

use crate::app::menu::options::MenuOptions;

pub type NivaSize = LogicalSize<f64>;
pub type NivaPosition = LogicalPosition<f64>;

#[derive(Deserialize, Clone, Debug)]
pub struct WindowRootMenu {
    pub label: String,
    pub enabled: Option<bool>,
    pub children: MenuOptions
}

pub type WindowMenuOptions = Vec<WindowRootMenu>;


#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct NivaWindowOptions {
    pub entry: Option<String>,
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

    // merge background_color options to transparent
    pub menu: Option<WindowMenuOptions>,
}


