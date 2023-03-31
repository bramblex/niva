
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum NativeLabel {
    Hide,
    Services,
    HideOthers,
    ShowAll,
    CloseWindow,
    Quit,
    Copy,
    Cut,
    Undo,
    Redo,
    SelectAll,
    Paste,
    EnterFullScreen,
    Minimize,
    Zoom,
    Separator,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MenuItemOption {
    Native {
        label: NativeLabel,
    },
    Item {
        id: u16,
        label: String,
        enabled: Option<bool>,
        selected: Option<bool>,
        icon: Option<String>,
        accelerator: Option<String>,
    },
    Menu {
        label: String,
        enabled: Option<bool>,
        children: MenuOptions,
    },
}

pub type MenuOptions = Vec<MenuItemOption>;