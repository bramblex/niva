use crate::window_manager::options::WindowOptions;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub icon: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
    pub website: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectOptions {
    // project config
    pub name: String,
    pub uuid: Option<String>,

    // meta config
    #[serde(flatten)]
    pub meta: Meta,

    // window options
    #[serde(flatten)]
    pub window: WindowOptions,
}
