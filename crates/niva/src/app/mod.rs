mod options;
mod utils;
mod arguments;
mod launch_info;

use std::sync::Arc;
use anyhow::Result;
use launch_info::NivaLaunchInfo;

pub struct NivaApp {
    pub launch_info: NivaLaunchInfo,
}

impl NivaApp {
    pub fn new() -> Result<Arc<NivaApp>> {
        let launch_info = NivaLaunchInfo::new()?;
        todo!("")
    }
}