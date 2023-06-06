use std::sync::{Arc, Mutex};

use super::{
    event::{NivaEvent, NivaEventLoop, NivaWindowTarget},
    launch_info::NivaLaunchInfo,
    NivaAppRef,
};
use crate::utils::arc_mut::ArcMut;
use anyhow::Result;
use tao::{event::Event, event_loop::ControlFlow};

pub struct NivaShortcutManager {}

impl NivaShortcutManager {
    pub fn new(launch_info: &NivaLaunchInfo) -> Result<ArcMut<NivaShortcutManager>> {
        Ok(Arc::new(Mutex::new(Self {})))
    }

    pub async fn init(&mut self, app: &NivaAppRef) -> Result<()> {
        Ok(())
    }

    pub fn start(&mut self, event_loop: &NivaEventLoop) -> Result<()> {
        Ok(())
    }

    pub fn run(
        &mut self,
        event: &Event<NivaEvent>,
        target: &NivaWindowTarget,
        control_flow: &mut ControlFlow,
    ) -> Result<()> {
        Ok(())
    }
}
