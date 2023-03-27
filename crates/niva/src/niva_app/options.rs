use serde::Deserialize;
use super::window_manager::options::NivaWindowOptions;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaOptions {
    // base options
    pub name: String,
    pub uuid: String,
    pub icon: Option<String>,

    // window options
    #[serde(default)]
    pub window: NivaWindowOptions,

    // app options
    pub activation: Option<NivaActivationPolicy>, // (MacOS Only)Activation policy of the application.
    pub tray: Option<NivaTrayOptions>,
    pub workers: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub enum NivaActivationPolicy {
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "accessory")]
    Accessory,
    #[serde(rename = "prohibited")]
    Prohibited,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MenuItemOptions {
    NativeItem(String),
    MenuItem(String, u16),
    SubMenu(String, Vec<MenuItemOptions>),
}

#[derive(Deserialize, Debug)]
pub struct MenuOptions(pub Vec<MenuItemOptions>);

#[derive(Deserialize, Debug)]
pub struct NivaTrayOptions {
    pub icon: String,
    pub title: Option<String>,
    pub tooltip: Option<String>,
    pub menu: Option<MenuOptions>,
}
