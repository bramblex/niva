use std::{any::Any, sync::Arc};
use tao::event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};

pub type NivaEvent = Box<dyn Any + Send + Sync>;
pub type NivaEventLoop = EventLoop<NivaEvent>;
pub type NivaEventLoopProxy = EventLoopProxy<NivaEvent>;
pub type NivaWindowTarget = EventLoopWindowTarget<NivaEvent>;
