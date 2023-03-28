use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum NivaMenu {
    #[serde(rename = "native")]
    Native { label: String },
    #[serde(rename = "item")]
    Item { label: String, id: u16 },
    #[serde(rename = "menu")]
    Menu {
        label: String,
        children: Vec<NivaMenu>,
    },
}

#[derive(Deserialize, Serialize)]
enum RootMenuType {
    #[serde(rename = "menu")]
    Menu,
}

#[derive(Deserialize, Serialize)]
pub struct RootMenu {
    r#type: RootMenuType,
    pub label: String,
    pub children: Vec<NivaMenu>,
}
