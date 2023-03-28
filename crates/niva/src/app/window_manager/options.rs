use serde::Deserialize;
use crate::app::options::MenuOptions;

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Size(pub f64, pub f64);

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Position(pub f64, pub f64);

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct NivaWindowOptions {
    pub entry: Option<String>,
    pub parent: Option<u32>,
    pub devtools: Option<bool>,

    pub title: Option<String>,
    pub icon: Option<String>,
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

    // merge background_color options to transparent
    pub menu: Option<MenuOptions>,
}


