use anyhow::Result;
use std::{collections::HashMap, sync::Arc};

pub struct NivaApiArguments {
}

pub trait NivaApi {
    fn name(&self) -> String;
    fn invoke(&self, method: &str, args: NivaApiArguments) -> Result<()>;
}

pub struct NivaApiManager {
    api_instance: HashMap<String, Arc<dyn NivaApi>>,
}

impl NivaApiManager {
    pub fn register(&self) {
    }
}