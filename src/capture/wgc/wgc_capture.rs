use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use windows::core::IInspectable;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
};

use windows::Graphics::DirectX::DirectXPixelFormat;

use crate::capture::wgc::d3d;
use crate::capture::YuvConverter;
use crate::config::Config;
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::{OutputSink, ScreenCapture};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};

pub struct WGCScreenCapture<'a> {
    item: GraphicsCaptureItem,
    device: Arc<ID3D11Device>,
    d3d_context: Arc<ID3D11DeviceContext>,
    frame_pool: Direct3D11CaptureFramePool,
    config: &'a Config,
    duplicator: YuvConverter,
}

impl<'a> WGCScreenCapture<'a> {
    pub fn new(item: GraphicsCaptureItem, config: &'a Config) -> Result<Self> {
        let item_size = item.Size()?;
        let (device, d3d_device, d3d_context) = d3d::create_direct3d_devices_and_context()?;
        let device = Arc::new(device);
        let d3d_context = Arc::new(d3d_context);
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            item_size,
        )?;
        let duplicator = YuvConverter::new(
            device.clone(),
            d3d_context.clone(),
            (item_size.Width as u32, item_size.Height as u32),
        )?;
        Ok(Self {
            item,
            device,
            d3d_context,
            frame_pool,
            config,
            duplicator,
        })
    }
}

#[async_trait]
impl ScreenCapture for WGCScreenCapture<'_> {
    async fn capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        mut output: Box<impl OutputSink + Send + ?Sized>,
        mut profiler: PerformanceProfiler,
    ) -> Result<()> {
        let session = self.frame_pool.CreateCaptureSession(&self.item)?;

        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Direct3D11CaptureFrame>(1);

        self.frame_pool.FrameArrived(&TypedEventHandler::<
            Direct3D11CaptureFramePool,
            IInspectable,
        >::new({
            move |frame_pool, _| {
                let frame_pool = frame_pool.as_ref().unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                sender.try_send(frame).unwrap();
                Ok(())
            }
        }))?;

        session.StartCapture()?;

        let mut ticker =
            tokio::time::interval(Duration::from_millis((1000 / self.config.max_fps) as u64));

        while let Some(frame) = receiver.recv().await {
            let frame_time = frame.SystemRelativeTime()?.Duration;
            profiler.accept_frame(frame.SystemRelativeTime()?.Duration);
            let yuv_frame = unsafe {
                let source_texture: ID3D11Texture2D =
                    d3d::get_d3d_interface_from_object(&frame.Surface()?)?;
                self.duplicator.capture(source_texture)?
            };
            profiler.done_preprocessing();
            let encoded = encoder
                .encode(FrameData::NV12(&yuv_frame), frame_time)
                .unwrap();
            let encoded_len = encoded.len();
            profiler.done_encoding();
            output.write(encoded).await.unwrap();
            profiler.done_processing(encoded_len);
            ticker.tick().await;
        }
        session.Close()?;
        Ok(())
    }
}

impl Drop for WGCScreenCapture<'_> {
    fn drop(&mut self) {
        self.frame_pool.Close().unwrap();
    }
}
