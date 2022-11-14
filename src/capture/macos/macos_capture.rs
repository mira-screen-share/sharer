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

use crate::capture::d3d;
use crate::config::Config;
use crate::encoder::FfmpegEncoder;
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::{OutputSink, ScreenCapture};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_BIND_FLAG,
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_RESOURCE_MISC_FLAG, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING,
};

pub struct MacOSScreenCapture<'a> {
    item: GraphicsCaptureItem,
    device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    frame_pool: Direct3D11CaptureFramePool,
    config: &'a Config,
}

impl<'a> MacOSScreenCapture<'a> {
    pub fn new(item: GraphicsCaptureItem, config: &'a Config) -> Result<Self> {
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
            config,
        })
    }
}

#[async_trait]
impl ScreenCapture for MacOSScreenCapture<'_> {
    async fn capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        mut output: Box<impl OutputSink + Send + ?Sized>,
        mut profiler: PerformanceProfiler,
    ) -> Result<()> {

    }
}

impl Drop for MacOSScreenCapture<'_> {
    fn drop(&mut self) {

    }
}
