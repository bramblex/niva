use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaOptions {
    // base options
    pub name: String,
    pub id: String,
    pub icon: Option<String>,
}
