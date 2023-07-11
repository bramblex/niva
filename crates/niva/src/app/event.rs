use tao::event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};

pub type NivaEvent = ();
pub type NivaEventLoop = EventLoop<NivaEvent>;
pub type NivaEventLoopProxy = EventLoopProxy<NivaEvent>;
pub type NivaWindowTarget = EventLoopWindowTarget<NivaEvent>;
