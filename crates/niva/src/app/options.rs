use serde::Deserialize;

use super::resource::options::ResourceOptions;

#[derive(Deserialize, Clone, Debug)]
pub enum NivaActivationPolicy {
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "accessory")]
    Accessory,
    #[serde(rename = "prohibited")]
    Prohibited,
}

#[cfg(target_os = "macos")]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MacExtraOptions {
    pub activation_policy: Option<NivaActivationPolicy>, // (MacOS Only)Activation policy of the application.
    pub default_menu_creation: Option<bool>,
    pub activate_ignoring_other_apps: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaOptions {
    // base options
    pub name: String,
    pub id: String,

    #[serde(default)]
    pub resource: ResourceOptions,

    // pub icon: Option<String>,
    // window options
    // #[serde(default)]
    // pub window: NivaWindowOptions,
    // pub tray: Option<NivaTrayOptions>,
    // pub shortcuts: Option<NivaShortcutsOptions>,

    // mac app options
    #[cfg(target_os = "macos")]
    #[serde(flatten)]
    pub macos_extra: Option<MacExtraOptions>,
}
