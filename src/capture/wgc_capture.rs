use std::slice;
use std::sync::mpsc::channel;
use windows::core::{IInspectable, Interface};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DSurface;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::{D3D11_BIND_FLAG, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_RESOURCE_MISC_FLAG, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D};
use crate::result::Result;
use crate::{d3d, OutputSink};
use crate::encoder::Encoder;
use super::ScreenCapture;

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

pub struct BGR0YUVConverter {
    yuv: Vec<u8>,
    width: usize,
    height: usize,
}

impl BGR0YUVConverter {
    /// Allocates a new helper for the given format.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            yuv: vec![0u8; (3 * (width * height)) / 2],
            width,
            height,
        }
    }

    /// Converts the RGB array.
    pub fn convert(&mut self, rgb: &[u8]) {
        let width = self.width;
        let height = self.height;

        let u_base = width * height;
        let v_base = u_base + u_base / 4;
        let half_width = width / 2;

        assert_eq!(rgb.len(), width * height * 4);
        assert_eq!(width % 2, 0, "width needs to be multiple of 2");
        assert_eq!(height % 2, 0, "height needs to be a multiple of 2");

        // y is full size, u, v is quarter size
        let pixel = |x: usize, y: usize| -> (f32, f32, f32) {
            // two dim to single dim
            let base_pos = (x + y * width) * 4;
            (rgb[base_pos + 2] as f32, rgb[base_pos + 1] as f32, rgb[base_pos] as f32)
        };

        let write_y = |yuv: &mut [u8], x: usize, y: usize, rgb: (f32, f32, f32)| {
            yuv[x + y * width] = (0.2578125 * rgb.0 + 0.50390625 * rgb.1 + 0.09765625 * rgb.2 + 16.0) as u8;
        };

        let write_u = |yuv: &mut [u8], x: usize, y: usize, rgb: (f32, f32, f32)| {
            yuv[u_base + x + y * half_width] = (-0.1484375 * rgb.0 + -0.2890625 * rgb.1 + 0.4375 * rgb.2 + 128.0) as u8;
        };

        let write_v = |yuv: &mut [u8], x: usize, y: usize, rgb: (f32, f32, f32)| {
            yuv[v_base + x + y * half_width] = (0.4375 * rgb.0 + -0.3671875 * rgb.1 + -0.0703125 * rgb.2 + 128.0) as u8;
        };

        for i in 0..width / 2 {
            for j in 0..height / 2 {
                let px = i * 2;
                let py = j * 2;
                let pix0x0 = pixel(px, py);
                let pix0x1 = pixel(px, py + 1);
                let pix1x0 = pixel(px + 1, py);
                let pix1x1 = pixel(px + 1, py + 1);
                let avg_pix = (
                    (pix0x0.0 as u32 + pix0x1.0 as u32 + pix1x0.0 as u32 + pix1x1.0 as u32) as f32 / 4.0,
                    (pix0x0.1 as u32 + pix0x1.1 as u32 + pix1x0.1 as u32 + pix1x1.1 as u32) as f32 / 4.0,
                    (pix0x0.2 as u32 + pix0x1.2 as u32 + pix1x0.2 as u32 + pix1x1.2 as u32) as f32 / 4.0,
                );
                write_y(&mut self.yuv[..], px, py, pix0x0);
                write_y(&mut self.yuv[..], px, py + 1, pix0x1);
                write_y(&mut self.yuv[..], px + 1, py, pix1x0);
                write_y(&mut self.yuv[..], px + 1, py + 1, pix1x1);
                write_u(&mut self.yuv[..], i, j, avg_pix);
                write_v(&mut self.yuv[..], i, j, avg_pix);
            }
        }
    }

    fn width(&self) -> i32 {
        self.width as i32
    }

    fn height(&self) -> i32 {
        self.height as i32
    }

    fn y(&self) -> &[u8] {
        &self.yuv[0..self.width * self.height]
    }

    fn u(&self) -> &[u8] {
        let base_u = self.width * self.height;
        &self.yuv[base_u..base_u + base_u / 4]
    }

    fn v(&self) -> &[u8] {
        let base_u = self.width * self.height;
        let base_v = base_u + base_u / 4;
        &self.yuv[base_v..]
    }

    fn y_stride(&self) -> i32 {
        self.width as i32
    }

    fn u_stride(&self) -> i32 {
        (self.width / 2) as i32
    }

    fn v_stride(&self) -> i32 {
        (self.width / 2) as i32
    }
}

impl ScreenCapture for WGCScreenCapture<'_> {
    fn capture(&mut self, encoder: &mut dyn Encoder, output: &mut dyn OutputSink) -> Result<()> {
        let session = self.frame_pool.CreateCaptureSession(self.item)?;

        let (sender, receiver) = channel();

        self.frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                move |frame_pool, _| {
                    let frame_pool = frame_pool.as_ref().unwrap();
                    let frame = frame_pool.TryGetNextFrame()?;
                    sender.send(frame).unwrap();
                    Ok(())
                }
            }),
        )?;

        session.StartCapture()?;

        let height = self.item.Size()?.Height as u32;
        let width = self.item.Size()?.Width as u32;
        let mut yuv_converter = BGR0YUVConverter::new(width as usize, height as usize);
        while let Ok(frame) = receiver.recv() {
            unsafe {
                let texture = self.surface_to_texture(&frame.Surface()?)?;
                let resource: ID3D11Resource = texture.cast()?;
                let mapped = self.d3d_context.Map(&resource, 0, D3D11_MAP_READ, 0)?;
                let frame: &[u8] = slice::from_raw_parts(
                    mapped.pData as *const _,
                    (height * mapped.RowPitch) as usize,
                );
                yuv_converter.convert(frame);
                let encoded = encoder.encode(yuv_converter.y(), yuv_converter.u(), yuv_converter.v()).unwrap();
                output.write(encoded).unwrap();
                self.d3d_context.Unmap(&resource, 0);
            };
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

