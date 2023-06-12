mod options;
mod arguments;
mod resource;
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

        Ok(Arc::new(Self {
            launch_info
        }))
    }
}