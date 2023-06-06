use tao::event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};

pub type NivaEventLoop = EventLoop<NivaEvent>;
pub type NivaEventLoopProxy = EventLoopProxy<NivaEvent>;
pub type NivaWindowTarget = EventLoopWindowTarget<NivaEvent>;

use serde_json::Value as JsonValue;

#[derive(Debug)]
pub enum NivaEvent {
    // IpcEvent
    IpcReceiveEvent {
        window_id: u8,
        event: String,
        data: JsonValue,
    },
    IpcSendEvent {
        window_id: u8,
        event: String,
        data: JsonValue,
    },
}
