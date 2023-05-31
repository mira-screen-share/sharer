use async_trait::async_trait;
use bytes::Bytes;

use crate::Result;

#[async_trait]
pub trait OutputSink: Send + Sync + 'static {
    async fn write(&mut self, input: Bytes) -> Result<()>;
    async fn write_audio(&mut self, input: Bytes) -> Result<()>;
}

mod file_output;
mod webrtc_output;
mod webrtc_peer;

pub use file_output::FileOutput;
pub use webrtc_output::WebRTCOutput;
pub use webrtc_peer::WebRTCPeer;
