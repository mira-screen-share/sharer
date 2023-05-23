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

pub trait DisplayInfo {
    /// Get the resolution of the display in (width, height)
    fn resolution(&self) -> (u32, u32);
    /// Get the DPI factor for input handling
    fn dpi_conversion_factor(&self) -> f64;
}

use crate::encoder::FfmpegEncoder;
use crate::performance_profiler::PerformanceProfiler;

#[cfg(target_os = "windows")]
mod wgc;
#[cfg(target_os = "windows")]
pub use wgc::display::Display;
#[cfg(target_os = "windows")]
pub use wgc::WGCScreenCapture as ScreenCaptureImpl;

mod frame;
#[cfg(target_os = "macos")]
mod macos;

pub use frame::YUVFrame;
#[cfg(target_os = "macos")]
pub use macos::display::Display;
#[cfg(target_os = "macos")]
pub use macos::MacOSScreenCapture as ScreenCaptureImpl;

mod yuv_convert;
pub use yuv_convert::YuvConverter;
