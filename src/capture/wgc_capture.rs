use async_trait::async_trait;
use std::slice;
use std::time::Duration;
use windows::core::{IInspectable, Interface};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DSurface;
use windows::Graphics::DirectX::DirectXPixelFormat;

use super::ScreenCapture;
use crate::capture::d3d;
use crate::capture::yuv_converter::BGR0YUVConverter;
use crate::encoder::Encoder;
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::OutputSink;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_BIND_FLAG,
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_RESOURCE_MISC_FLAG, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING,
};

pub struct WGCScreenCapture<'a> {
    item: &'a GraphicsCaptureItem,
    device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    frame_pool: Direct3D11CaptureFramePool,
}

impl<'a> WGCScreenCapture<'a> {
    unsafe fn surface_to_texture(&mut self, surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
        let source_texture: ID3D11Texture2D = d3d::get_d3d_interface_from_object(surface)?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        source_texture.GetDesc(&mut desc);
        desc.BindFlags = D3D11_BIND_FLAG(0);
        desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);
        desc.Usage = D3D11_USAGE_STAGING;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
        let copy_texture = self.device.CreateTexture2D(&desc, None)?;
        let src: ID3D11Resource = source_texture.cast()?;
        let dst: ID3D11Resource = copy_texture.cast()?;
        self.d3d_context.CopyResource(&dst, &src);
        Ok(copy_texture)
    }

    unsafe fn get_frame_content(
        &mut self,
        frame: Direct3D11CaptureFrame,
    ) -> Result<(ID3D11Resource, &[u8])> {
        let texture = self.surface_to_texture(&frame.Surface()?)?;
        let resource: ID3D11Resource = texture.cast()?;
        let mapped = self.d3d_context.Map(&resource, 0, D3D11_MAP_READ, 0)?;
        let frame: &[u8] = slice::from_raw_parts(
            mapped.pData as *const _,
            (self.item.Size()?.Height as u32 * mapped.RowPitch) as usize,
        );
        Ok((resource, frame))
    }

    pub fn new(item: &'a GraphicsCaptureItem) -> Result<Self> {
        let item_size = item.Size()?;
        let (device, d3d_device, d3d_context) = d3d::create_direct3d_devices_and_context()?;
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            item_size,
        )?;
        Ok(Self {
            item,
            device,
            d3d_context,
            frame_pool,
        })
    }
}

#[async_trait]
impl ScreenCapture for WGCScreenCapture<'_> {
    async fn capture(
        &mut self,
        mut encoder: Box<impl Encoder + Send>,
        mut output: Box<impl OutputSink + Send>,
    ) -> Result<()> {
        let session = self.frame_pool.CreateCaptureSession(self.item)?;

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

        let height = self.item.Size()?.Height as u32;
        let width = self.item.Size()?.Width as u32;
        let mut profiler = PerformanceProfiler::new();
        let mut yuv_converter = BGR0YUVConverter::new(width as usize, height as usize);
        let mut ticker = tokio::time::interval(Duration::from_millis(33)); // 30 FPS
        while let Some(frame) = receiver.recv().await {
            profiler.accept_frame(frame.SystemRelativeTime()?.Duration);
            let (resource, frame) = unsafe { self.get_frame_content(frame)? };
            profiler.done_preprocessing();
            yuv_converter.convert(frame);
            profiler.done_conversion();
            let encoded = encoder
                .encode(yuv_converter.y(), yuv_converter.u(), yuv_converter.v())
                .unwrap();
            profiler.done_encoding();
            output.write(encoded).await.unwrap();
            unsafe {
                self.d3d_context.Unmap(&resource, 0);
            }
            profiler.done_processing();
            debug!("{}", profiler.generate_report(encoded.len()));
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
