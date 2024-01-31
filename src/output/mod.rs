use async_trait::async_trait;
use bytes::Bytes;
use std::time::Duration;

use crate::Result;

#[async_trait]
pub trait OutputSink: Send + Sync + 'static {
    async fn write(&mut self, input: Bytes) -> Result<()>;
    async fn write_audio(&mut self, input: Bytes, duration: Duration) -> Result<()>;
}

mod file_output;
mod noop_output;
mod webrtc_output;
mod webrtc_peer;

pub use file_output::FileOutput;
#[allow(unused_imports)]
pub use noop_output::NoOpOutput;
pub use webrtc_output::WebRTCOutput;
pub use webrtc_peer::WebRTCPeer;
