use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub enum SubMenuItemOptions {
    Native(MenuNativeItemOptions),
    Item(MenuItemOptions),
    Menu(MenuOptions),
}

#[derive(Deserialize, Debug)]
pub struct MenuItemOptions {
    id: u8,
    label: String,
    enabled: Option<bool>,
    selected: Option<bool>,
    icon: Option<String>,
    accelerator: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MenuOptions {
    label: String,
    enabled: Option<bool>,
    children: Vec<SubMenuItemOptions>,
}

#[derive(Deserialize, Debug)]
pub struct MenuNativeItemOptions {
    label: NativeLabel,
}

#[derive(Deserialize, Debug)]
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
