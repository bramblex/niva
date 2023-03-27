use serde::Deserialize;
use crate::niva_app::options::MenuOptions;

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Size(pub f64, pub f64);

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Position(pub f64, pub f64);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaWindowOptions {
    pub entry: Option<String>,

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

impl Default for NivaWindowOptions {
    fn default() -> Self {
        Self {
            entry: None,
            title: None,
            icon: None,
            theme: None,
            size: None,
            min_size: None,
            max_size: None,
            position: None,
            resizable: None,
            minimizable: None,
            maximizable: None,
            closable: None,
            fullscreen: None,
            maximized: None,
            visible: None,
            transparent: None,
            decorations: None,
            always_on_top: None,
            always_on_bottom: None,
            visible_on_all_workspaces: None,
            focused: None,
            content_protection: None,
            menu: None,
        }
    }
}
