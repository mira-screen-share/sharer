use crate::{OutputSink, Result};
use async_trait::async_trait;

#[async_trait]
pub trait ScreenCapture {
    async fn capture(
        &mut self,
        mut encoder: Box<impl Encode + Send>,
        mut output: Box<impl OutputSink + Send + ?Sized>,
        mut profiler: PerformanceProfiler,
        max_fps: u32,
    ) -> Result<()>;
}

mod d3d;
pub mod display;
mod wgc_capture;
mod yuv_converter;

use crate::encoder::Encode;
use crate::performance_profiler::PerformanceProfiler;
pub use wgc_capture::WGCScreenCapture;
