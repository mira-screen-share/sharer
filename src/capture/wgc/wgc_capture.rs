use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use windows::core::IInspectable;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};

use windows::Graphics::DirectX::DirectXPixelFormat;

use crate::capture::display::DisplaySelector;
use crate::capture::wgc::d3d;
use crate::capture::wgc::display::Display;
use crate::capture::{DisplayInfo, ScreenCaptureImpl, YuvConverter};
use crate::config::Config;
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::{OutputSink, ScreenCapture};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct WGCScreenCapture {
    config: Config,
    engine: CaptureEngine,
    selected_display: Display,
    session: Option<GraphicsCaptureSession>,
}

struct CaptureEngine {
    item: GraphicsCaptureItem,
    frame_pool: Direct3D11CaptureFramePool,
    duplicator: YuvConverter,
}

impl CaptureEngine {
    fn new(item: GraphicsCaptureItem) -> Self {
        let item_size = item.Size().unwrap();
        let (device, d3d_device, d3d_context) = d3d::create_direct3d_devices_and_context().unwrap();
        let device = Arc::new(device);
        let d3d_context = Arc::new(d3d_context);
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            3,
            item_size,
        )
        .unwrap();
        let duplicator = YuvConverter::new(
            device,
            d3d_context,
            (item_size.Width as u32, item_size.Height as u32),
        )
        .unwrap();
        Self {
            item,
            frame_pool,
            duplicator,
        }
    }
}

#[async_trait]
impl ScreenCapture for WGCScreenCapture {
    fn new(config: Config) -> Result<ScreenCaptureImpl> {
        let selected_display = Display::online().unwrap()[0].clone();
        let item = selected_display.select()?;
        let engine = CaptureEngine::new(item);
        Ok(Self {
            config,
            engine,
            selected_display,
            session: None,
        })
    }

    fn display(&self) -> &dyn DisplayInfo {
        &self.engine.item
    }

    async fn start_capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        output: Arc<Mutex<impl OutputSink + Send + ?Sized>>,
        mut profiler: PerformanceProfiler,
        shutdown_token: CancellationToken,
    ) -> Result<()> {
        let session = self
            .engine
            .frame_pool
            .CreateCaptureSession(&self.engine.item)?;

        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Direct3D11CaptureFrame>(1);

        self.engine.frame_pool.FrameArrived(&TypedEventHandler::<
            Direct3D11CaptureFramePool,
            IInspectable,
        >::new({
            move |frame_pool, _| {
                let frame_pool = frame_pool.as_ref().unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                sender
                    .try_send(frame)
                    .unwrap_or_else(move |err| warn!("Failed to send frame: {}", err.to_string()));
                Ok(())
            }
        }))?;

        session.StartCapture()?;
        self.session.replace(session);

        let mut duplicator = self.engine.duplicator.clone();

        let mut ticker =
            tokio::time::interval(Duration::from_millis((1000 / self.config.max_fps) as u64));

        tokio::spawn(async move {
            loop {
                select! {
                    Some(frame) = receiver.recv() => {
                        let frame_time = frame.SystemRelativeTime().unwrap().Duration;
                        profiler.accept_frame(frame.SystemRelativeTime().unwrap().Duration);
                        let yuv_frame = {
                            duplicator
                                .capture(d3d::get_d3d_interface_from_object(&frame.Surface().unwrap()).unwrap()).unwrap()
                        };
                        profiler.done_preprocessing();
                        let encoded = encoder
                            .encode(FrameData::NV12(&yuv_frame), frame_time)
                            .unwrap();
                        let encoded_len = encoded.len();
                        profiler.done_encoding();
                        output.lock().await.write(encoded).await.unwrap();
                        profiler.done_processing(encoded_len);
                        ticker.tick().await;
                    }
                    _ = shutdown_token.cancelled() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<()> {
        if let Some(session) = self.session.take() {
            session.Close()?;
        }
        Ok(())
    }
}

impl DisplaySelector for WGCScreenCapture {
    type Display = Display;

    fn available_displays(&mut self) -> Result<Vec<Display>> {
        Display::online()
    }

    fn select_display(&mut self, display: &Display) -> Result<()> {
        self.engine = CaptureEngine::new(display.select()?);
        self.selected_display = display.clone();
        Ok(())
    }

    fn selected_display(&self) -> Result<Option<Self::Display>> {
        Ok(Some(self.selected_display.clone()))
    }
}

impl Drop for CaptureEngine {
    fn drop(&mut self) {
        self.frame_pool.Close().unwrap();
    }
}
