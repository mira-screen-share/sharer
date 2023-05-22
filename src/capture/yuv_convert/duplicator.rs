use super::{
    dx_math::{VERTEX_STRIDES, VERTICES},
    shader,
};
use crate::capture::yuv_convert::dx_math::Vertex;
use crate::capture::YUVFrame;
use crate::encoder::FrameData;
use crate::result::Result;
use clap::__macro_refs::once_cell;
use renderdoc::{RenderDoc, V110};
use std::os::raw::c_void;
use std::rc::Rc;
use std::slice;
use std::sync::Arc;
use windows::{
    core::{Interface, PCSTR, PCWSTR},
    Win32::{
        Graphics::{
            Direct3D::*,
            Direct3D11::*,
            Dxgi::{Common::*, *},
            Gdi::*,
        },
        UI::WindowsAndMessaging::*,
    },
};

pub struct Duplicator {
    device: Arc<ID3D11Device>,
    device_context: Arc<ID3D11DeviceContext>,

    vertex_shader: ID3D11VertexShader,
    vertex_buffer: Option<ID3D11Buffer>,

    pixel_shader_luminance: ID3D11PixelShader,
    pixel_shader_chrominance: ID3D11PixelShader,

    backend_texture: ID3D11Texture2D,
    backend_viewport: [D3D11_VIEWPORT; 1],
    backend_rtv: [Option<ID3D11RenderTargetView>; 1],

    luminance_render_texture: ID3D11Texture2D,
    luminance_staging_texture: ID3D11Texture2D,
    luminance_viewport: [D3D11_VIEWPORT; 1],
    luminance_rtv: [Option<ID3D11RenderTargetView>; 1],

    chrominance_render_texture: ID3D11Texture2D,
    chrominance_staging_texture: ID3D11Texture2D,
    chrominance_viewport: [D3D11_VIEWPORT; 1],
    chrominance_rtv: [Option<ID3D11RenderTargetView>; 1],

    resolution: (u32, u32),
    //rd: RenderDoc<V110>,
}

unsafe impl Send for Duplicator {}

impl Duplicator {
    pub fn new(
        device: Arc<ID3D11Device>,
        device_context: Arc<ID3D11DeviceContext>,
        resolution: (u32, u32),
    ) -> Result<Duplicator> {
        unsafe {
            let (backend_texture, backend_rtv, backend_viewport) =
                init_backend_resources(&device, resolution)?;

            let (vertex_shader, vertex_buffer, pixel_shader_lumina, pixel_shader_chrominance) =
                init_shaders(&device)?;

            let (lumina_render_texture, lumina_staging_texture, lumina_viewport, lumina_rtv) =
                init_lumina_resources(&device, resolution)?;

            let (
                chrominance_render_texture,
                chrominance_staging_texture,
                chrominance_viewport,
                chrominance_rtv,
            ) = init_chrominance_resources(&device, resolution)?;

            let sampler_state = init_sampler_state(&device)?;

            device_context.PSSetSamplers(0, Some(&[Some(sampler_state)]));

            device_context.IASetInputLayout(&init_input_layout(&device)?);

            Ok(Duplicator {
                device,
                device_context,
                vertex_shader,
                vertex_buffer: Some(vertex_buffer),
                pixel_shader_luminance: pixel_shader_lumina,
                pixel_shader_chrominance,
                backend_texture,
                backend_viewport: [backend_viewport],
                backend_rtv: [Some(backend_rtv)],
                luminance_render_texture: lumina_render_texture,
                luminance_staging_texture: lumina_staging_texture,
                luminance_viewport: [lumina_viewport],
                luminance_rtv: [Some(lumina_rtv)],
                chrominance_render_texture,
                chrominance_staging_texture,
                chrominance_viewport: [chrominance_viewport],
                chrominance_rtv: [Some(chrominance_rtv)],
                resolution,
                //rd: RenderDoc::new().unwrap(),
            })
        }
    }

    fn inspect_texture(&self, source_texture: &ID3D11Texture2D) -> Result<()> {
        unsafe {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            source_texture.GetDesc(&mut desc);

            println!("source_texture: {:?}", desc);
            desc.BindFlags = D3D11_BIND_FLAG(0);
            desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);
            desc.Usage = D3D11_USAGE_STAGING;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
            let copy_texture = self.device.CreateTexture2D(&desc, None)?;
            let src: ID3D11Resource = source_texture.cast()?;
            let dst: ID3D11Resource = copy_texture.cast()?;
            self.device_context.CopyResource(&dst, &src);

            let resource: ID3D11Resource = copy_texture.cast()?;
            let mapped = self.device_context.Map(&resource, 0, D3D11_MAP_READ, 0)?;
            let frame: &[u8] = slice::from_raw_parts(
                mapped.pData as *const _,
                (desc.Height as u32 * mapped.RowPitch) as usize,
            );
            println!("frame: {:?}", frame[..20].to_vec());
            Ok(())
        }
    }

    pub fn capture(&mut self, desktop_texture: ID3D11Texture2D) -> Result<YUVFrame> {
        unsafe {
            //self.rd
            //    .start_frame_capture(std::ptr::null(), std::ptr::null());
            self.device_context
                .CopyResource(&self.backend_texture, &desktop_texture);
            self.draw_lumina_and_chrominance_texture()?;
            let result = self.create_capture_frame(self.resolution);
            //self.rd
            //    .end_frame_capture(std::ptr::null(), std::ptr::null());
            result
        }
    }

    unsafe fn draw_lumina_and_chrominance_texture(&self) -> Result<()> {
        let mut backend_texture_desc = std::mem::zeroed();
        self.backend_texture.GetDesc(&mut backend_texture_desc);

        let shader_resouce_view_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: backend_texture_desc.Format,
            ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV {
                    MostDetailedMip: backend_texture_desc.MipLevels - 1,
                    MipLevels: backend_texture_desc.MipLevels,
                },
            },
        };

        let shader_resource_view = self
            .device
            .CreateShaderResourceView(&self.backend_texture, Some(&shader_resouce_view_desc))?;

        let shader_resource_view = [Some(shader_resource_view)];

        self.device_context
            .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

        self.device_context.IASetVertexBuffers(
            0,
            1,
            Some(&self.vertex_buffer),
            Some(&VERTEX_STRIDES),
            Some(&0),
        );

        self.device_context.VSSetShader(&self.vertex_shader, None);

        // draw lumina plane

        self.device_context
            .OMSetRenderTargets(Some(&self.luminance_rtv), None);

        self.device_context
            .PSSetShaderResources(0, Some(&shader_resource_view));

        self.device_context
            .PSSetShader(&self.pixel_shader_luminance, None);

        self.device_context
            .RSSetViewports(Some(&self.luminance_viewport));

        self.device_context.Draw(VERTICES.len() as u32, 0);

        // draw chrominance plane

        self.device_context
            .OMSetRenderTargets(Some(&self.chrominance_rtv), None);

        self.device_context
            .PSSetShaderResources(0, Some(&shader_resource_view));

        self.device_context
            .PSSetShader(&self.pixel_shader_chrominance, None);

        self.device_context
            .RSSetViewports(Some(&self.chrominance_viewport));

        self.device_context.Draw(VERTICES.len() as u32, 0);

        Ok(())
    }

    unsafe fn create_capture_frame(&self, resolution: (u32, u32)) -> Result<YUVFrame> {
        self.device_context.CopyResource(
            &self.luminance_staging_texture,
            &self.luminance_render_texture,
        );

        self.device_context.CopyResource(
            &self.chrominance_staging_texture,
            &self.chrominance_render_texture,
        );

        let lumina_mapped_resource =
            self.device_context
                .Map(&self.luminance_staging_texture, 0, D3D11_MAP_READ, 0)?;

        let luminance_stride = lumina_mapped_resource.RowPitch;

        let luminance_bytes = std::slice::from_raw_parts(
            lumina_mapped_resource.pData as *mut u8,
            (resolution.1 * luminance_stride) as usize,
        )
        .to_vec();

        self.device_context
            .Unmap(&self.luminance_staging_texture, 0);

        let chrominance_mapped_resource =
            self.device_context
                .Map(&self.chrominance_staging_texture, 0, D3D11_MAP_READ, 0)?;

        let chrominance_stride = chrominance_mapped_resource.RowPitch;

        let chrominance_bytes = std::slice::from_raw_parts(
            chrominance_mapped_resource.pData as *mut u8,
            (resolution.1 / 2 * chrominance_stride) as usize,
        )
        .to_vec();

        self.device_context
            .Unmap(&self.chrominance_staging_texture, 0);

        Ok(YUVFrame {
            display_time: 0,
            width: resolution.0 as i32,
            height: resolution.1 as i32,
            luminance_bytes,
            luminance_stride: luminance_stride as i32,
            chrominance_bytes,
            chrominance_stride: chrominance_stride as i32,
        })
    }
}

unsafe fn init_shaders(
    device: &ID3D11Device,
) -> Result<(
    ID3D11VertexShader,
    ID3D11Buffer,
    ID3D11PixelShader,
    ID3D11PixelShader,
)> {
    let vertex_shader = device.CreateVertexShader(shader::VERTEX_SHADER_BYTES, None)?;

    let vertex_buffer_desc = D3D11_BUFFER_DESC {
        ByteWidth: VERTEX_STRIDES * VERTICES.len() as u32,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_VERTEX_BUFFER,
        CPUAccessFlags: D3D11_CPU_ACCESS_FLAG::default(),
        MiscFlags: D3D11_RESOURCE_MISC_FLAG::default(),
        StructureByteStride: 0,
    };

    let subresource_data = D3D11_SUBRESOURCE_DATA {
        pSysMem: &VERTICES as *const _ as *const c_void,
        SysMemPitch: 0,
        SysMemSlicePitch: 0,
    };

    let vertex_buffer = device.CreateBuffer(&vertex_buffer_desc, Some(&subresource_data))?;

    let pixel_shader_lumina = device.CreatePixelShader(shader::PIXEL_SHADER_LUMINA_BYTES, None)?;

    let pixel_shader_chrominance =
        device.CreatePixelShader(shader::PIXEL_SHADER_CHROMINANCE_BYTES, None)?;

    Ok((
        vertex_shader,
        vertex_buffer,
        pixel_shader_lumina,
        pixel_shader_chrominance,
    ))
}

unsafe fn init_lumina_resources(
    device: &ID3D11Device,
    resolution: (u32, u32),
) -> Result<(
    ID3D11Texture2D,
    ID3D11Texture2D,
    D3D11_VIEWPORT,
    ID3D11RenderTargetView,
)> {
    let mut texture_desc: D3D11_TEXTURE2D_DESC = std::mem::zeroed();
    texture_desc.Width = resolution.0;
    texture_desc.Height = resolution.1;
    texture_desc.MipLevels = 1;
    texture_desc.ArraySize = 1;
    texture_desc.Format = DXGI_FORMAT_R8_UNORM;
    texture_desc.SampleDesc.Count = 1;
    texture_desc.SampleDesc.Quality = 0;
    texture_desc.Usage = D3D11_USAGE_DEFAULT;
    texture_desc.BindFlags = D3D11_BIND_RENDER_TARGET;

    let render_texture = device.CreateTexture2D(&texture_desc, None)?;

    texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    texture_desc.Usage = D3D11_USAGE_STAGING;
    texture_desc.BindFlags = D3D11_BIND_FLAG::default();

    let staging_texture = device.CreateTexture2D(&texture_desc, None)?;

    let viewport = D3D11_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: resolution.0 as f32,
        Height: resolution.1 as f32,
        MinDepth: 0.0,
        MaxDepth: 1.0,
    };

    let rtv = device.CreateRenderTargetView(&render_texture, None)?;

    Ok((render_texture, staging_texture, viewport, rtv))
}

unsafe fn init_chrominance_resources(
    device: &ID3D11Device,
    resolution: (u32, u32),
) -> Result<(
    ID3D11Texture2D,
    ID3D11Texture2D,
    D3D11_VIEWPORT,
    ID3D11RenderTargetView,
)> {
    let mut texture_desc: D3D11_TEXTURE2D_DESC = std::mem::zeroed();
    texture_desc.Width = resolution.0 / 2;
    texture_desc.Height = resolution.1 / 2;
    texture_desc.MipLevels = 1;
    texture_desc.ArraySize = 1;
    texture_desc.Format = DXGI_FORMAT_R8G8_UNORM;
    texture_desc.SampleDesc.Count = 1;
    texture_desc.SampleDesc.Quality = 0;
    texture_desc.Usage = D3D11_USAGE_DEFAULT;
    texture_desc.BindFlags = D3D11_BIND_RENDER_TARGET;

    let render_texture = device.CreateTexture2D(&texture_desc, None)?;

    texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    texture_desc.Usage = D3D11_USAGE_STAGING;
    texture_desc.BindFlags = D3D11_BIND_FLAG::default();

    let staging_texture = device.CreateTexture2D(&texture_desc, None)?;

    let viewport = D3D11_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: (resolution.0 / 2) as f32,
        Height: (resolution.1 / 2) as f32,
        MinDepth: 0.0,
        MaxDepth: 1.0,
    };

    let rtv = device.CreateRenderTargetView(&render_texture, None)?;

    Ok((render_texture, staging_texture, viewport, rtv))
}

unsafe fn init_backend_resources(
    device: &ID3D11Device,
    resolution: (u32, u32),
) -> Result<(ID3D11Texture2D, ID3D11RenderTargetView, D3D11_VIEWPORT)> {
    let mut texture_desc: D3D11_TEXTURE2D_DESC = std::mem::zeroed();
    texture_desc.Width = resolution.0;
    texture_desc.Height = resolution.1;
    texture_desc.MipLevels = 1;
    texture_desc.ArraySize = 1;
    texture_desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    texture_desc.SampleDesc.Count = 1;
    texture_desc.SampleDesc.Quality = 0;
    texture_desc.Usage = D3D11_USAGE_DEFAULT;
    texture_desc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;

    let texture = device.CreateTexture2D(&texture_desc, None)?;

    texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    texture_desc.Usage = D3D11_USAGE_STAGING;
    texture_desc.BindFlags = D3D11_BIND_FLAG::default();

    let rtv = device.CreateRenderTargetView(&texture, None)?;

    let viewport = D3D11_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: resolution.0 as f32,
        Height: resolution.1 as f32,
        MinDepth: 0.0,
        MaxDepth: 1.0,
    };

    Ok((texture, rtv, viewport))
}

unsafe fn init_sampler_state(device: &ID3D11Device) -> Result<ID3D11SamplerState> {
    let mut sampler_desc: D3D11_SAMPLER_DESC = std::mem::zeroed();
    sampler_desc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
    sampler_desc.AddressU = D3D11_TEXTURE_ADDRESS_CLAMP;
    sampler_desc.AddressV = D3D11_TEXTURE_ADDRESS_CLAMP;
    sampler_desc.AddressW = D3D11_TEXTURE_ADDRESS_CLAMP;
    sampler_desc.ComparisonFunc = D3D11_COMPARISON_NEVER;
    sampler_desc.MinLOD = 0f32;
    sampler_desc.MaxLOD = D3D11_FLOAT32_MAX;

    let sampler_state = device.CreateSamplerState(&sampler_desc)?;

    Ok(sampler_state)
}

unsafe fn init_blend_state(device: &ID3D11Device) -> Result<ID3D11BlendState> {
    let mut blend_desc: D3D11_BLEND_DESC = std::mem::zeroed();
    blend_desc.AlphaToCoverageEnable = true.into();
    blend_desc.IndependentBlendEnable = false.into();
    blend_desc.RenderTarget[0].BlendEnable = true.into();
    blend_desc.RenderTarget[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
    blend_desc.RenderTarget[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
    blend_desc.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
    blend_desc.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_INV_DEST_ALPHA; //D3D11_BLEND_ONE ;
    blend_desc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ONE; //D3D11_BLEND_ZERO;
    blend_desc.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD; //D3D11_BLEND_OP_ADD;
    blend_desc.RenderTarget[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8;

    let blend_state = device.CreateBlendState(&blend_desc)?;

    Ok(blend_state)
}

unsafe fn init_input_layout(device: &ID3D11Device) -> Result<ID3D11InputLayout> {
    let input_element_desc_array = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: PCSTR(b"POSITION\0".as_ptr()),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: PCSTR(b"TEXCOORD\0".as_ptr()),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 12,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];

    let input_layout =
        device.CreateInputLayout(&input_element_desc_array, shader::VERTEX_SHADER_BYTES)?;

    Ok(input_layout)
}
