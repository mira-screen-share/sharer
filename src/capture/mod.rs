use crate::{OutputSink, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait ScreenCapture {
    fn new(config: Config) -> Result<ScreenCaptureImpl>;

    fn display(&self) -> &dyn DisplayInfo;

    async fn start_capture(
        &mut self,
        encoder: FfmpegEncoder,
        output: Arc<Mutex<impl OutputSink + Send + ?Sized>>,
        profiler: PerformanceProfiler,
        shutdown_token: CancellationToken,
    ) -> Result<()>;

    async fn stop_capture(&mut self) -> Result<()>;
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
pub use wgc::WGCScreenCapture as ScreenCaptureImpl;

pub mod capturer;
mod frame;
#[cfg(target_os = "macos")]
mod macos;

pub use frame::YUVFrame;
#[cfg(target_os = "macos")]
pub use macos::MacOSCapture as ScreenCaptureImpl;

mod audio;
pub mod display;
mod yuv_convert;

use crate::config::Config;
#[allow(unused_imports)]
pub use yuv_convert::YuvConverter;
