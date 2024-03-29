use super::{
    shortcut_manager::NivaShortcutsOptions, tray_manager::NivaTrayOptions,
    window_manager::options::NivaWindowOptions,
};
use serde::Deserialize;

#[cfg(target_os = "macos")]
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MacExtraOptions {
    pub activation_policy: Option<NivaActivationPolicy>, // (MacOS Only)Activation policy of the application.
    pub default_menu_creation: Option<bool>,
    pub activate_ignoring_other_apps: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaOptions {
    // base options
    pub name: String,
    pub uuid: String,
    pub icon: Option<String>,

    // window options
    #[serde(default)]
    pub window: NivaWindowOptions,

    pub tray: Option<NivaTrayOptions>,
    pub shortcuts: Option<NivaShortcutsOptions>,
    pub workers: Option<u32>,

    // mac app options
    #[cfg(target_os = "macos")]
    #[serde(flatten)]
    pub macos_extra: Option<MacExtraOptions>,
}

#[derive(Deserialize, Clone, Debug)]
pub enum NivaActivationPolicy {
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "accessory")]
    Accessory,
    #[serde(rename = "prohibited")]
    Prohibited,
}
