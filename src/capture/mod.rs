use crate::{OutputSink, Result};
use async_trait::async_trait;

#[async_trait]
pub trait ScreenCapture {
    async fn capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        mut output: Box<impl OutputSink + Send + ?Sized>,
        mut profiler: PerformanceProfiler,
    ) -> Result<()>;
}

mod d3d;
pub mod display;
mod wgc_capture;
mod yuv_converter;

use crate::encoder::FfmpegEncoder;
use crate::performance_profiler::PerformanceProfiler;
pub use wgc_capture::WGCScreenCapture;
pub use yuv_converter::BGR0YUVConverter;
