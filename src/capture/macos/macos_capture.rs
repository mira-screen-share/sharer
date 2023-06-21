use std::sync::Arc;
use std::time::Duration;

use anyhow::format_err;
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::capture::frame::YUVFrame;
use crate::capture::macos::screen_recorder::ScreenRecorder;
use crate::capture::Display;
use crate::config::Config;
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::{OutputSink, ScreenCapture};

pub struct MacOSScreenCapture<'a> {
    config: &'a Config,
}

unsafe impl Send for MacOSScreenCapture<'_> {}

pub type GraphicsCaptureItem = Display;

impl<'a> MacOSScreenCapture<'a> {
    pub fn new(_display: GraphicsCaptureItem, config: &'a Config) -> Result<Self> {
        // TODO select display
        // TODO hot-update config
        Ok(Self { config })
    }
}

#[async_trait]
impl ScreenCapture for MacOSScreenCapture<'_> {
    async fn capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        output: Arc<Mutex<impl OutputSink + Send + ?Sized>>,
        mut profiler: PerformanceProfiler,
    ) -> Result<()> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<YUVFrame>(1);

        let mut recorder = ScreenRecorder::new();
        recorder.start(sender).await;

        let mut ticker =
            tokio::time::interval(Duration::from_millis((1000 / self.config.max_fps) as u64));

        while let Some(frame) = receiver.recv().await {
            let frame_time = frame.display_time as f64;
            profiler.accept_frame(frame_time as i64);
            profiler.done_preprocessing();
            let encoded = encoder
                .encode(FrameData::NV12(&frame), frame_time as i64)
                .unwrap();
            let encoded_len = encoded.len();
            profiler.done_encoding();
            output.lock().await.write(encoded).await.unwrap();
            profiler.done_processing(encoded_len);
            ticker.tick().await;
        }

        Ok(())
    }
}
