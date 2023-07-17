use std::{
    collections::HashMap,
    sync::{Arc, Mutex}, any::Any,
};

use anyhow::Result;
use async_trait::async_trait;
use tao::{event::Event, event_loop::ControlFlow};

use super::{
    event::{NivaEvent, NivaEventLoop, NivaWindowTarget},
    NivaAppRef,
};

pub type NivaManagerRef = Arc<Mutex<dyn NivaManager + Send>>;

#[async_trait]
pub trait NivaManager {
    fn as_any(&mut self) -> &mut dyn Any;

    async fn init(&mut self, app: &NivaAppRef) -> Result<()>;
    fn start(&mut self, event_loop: &NivaEventLoop) -> Result<()>;
    fn tick(
        &mut self,
        event: &Event<NivaEvent>,
        target: &NivaWindowTarget,
        control_flow: *mut ControlFlow,
    ) -> Result<()> {
        Ok(())
    }
}
