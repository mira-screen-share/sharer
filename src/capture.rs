use crate::{Encoder, OutputSink, Result};
use async_trait::async_trait;

#[async_trait]
pub trait ScreenCapture {
    async fn capture(
        &mut self,
        mut encoder: Box<dyn Encoder + Send>,
        mut output: Box<dyn OutputSink + Send>,
    ) -> Result<()>;
}

mod wgc_capture;

pub use wgc_capture::WGCScreenCapture;
