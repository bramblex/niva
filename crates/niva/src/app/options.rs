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

    // API dispatcher options (fully async runtime; the old fixed
    // thread-pool `workers` setting is gone).
    pub api: Option<ApiOptions>,

    // dev options (also used as defaults when the matching CLI flags are absent)
    pub debug: Option<NivaDebugOptions>,

    // mac app options
    #[cfg(target_os = "macos")]
    #[serde(flatten)]
    pub macos_extra: Option<MacExtraOptions>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaDebugOptions {
    pub entry: Option<String>,
    pub resource: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiOptions {
    /// Per-request timeout in milliseconds (default 30000).
    pub timeout_ms: Option<u64>,
    /// Dispatch queue cap; overflow rejects immediately (default 64).
    pub max_queue: Option<usize>,
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
