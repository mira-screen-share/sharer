mod d3d;

use std::ffi::c_uchar;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::ptr::null_mut;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::{Duration, Instant};
use openh264::formats::{RBGYUVConverter, YUVSource};
use openh264_sys2::{ENCODER_OPTION_DATAFORMAT, SFrameBSInfo, SSourcePicture, videoFormatBGR, videoFormatI420};
use windows::core::{IInspectable, Interface, Result};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Foundation::TypedEventHandler;
use windows::Win32::Graphics::Direct3D11::{D3D11_BIND_FLAG, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_RESOURCE_MISC_FLAG, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, ID3D11Resource, ID3D11Texture2D};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::System::WinRT::{
    Graphics::Capture::IGraphicsCaptureItemInterop, RoInitialize, RO_INIT_MULTITHREADED,
};

#[derive(Clone)]
pub struct DisplayInfo {
    pub handle: HMONITOR,
    pub display_name: String,
}

impl DisplayInfo {
    pub fn new(monitor_handle: HMONITOR) -> Result<Self> {
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        unsafe {
            GetMonitorInfoW(monitor_handle, &mut info as *mut _ as *mut _).ok()?;
        }

        let display_name = String::from_utf16_lossy(&info.szDevice)
            .trim_matches(char::from(0))
            .to_string();

        Ok(Self {
            handle: monitor_handle,
            display_name,
        })
    }
}

fn create_capture_item_for_monitor(monitor_handle: HMONITOR) -> Result<GraphicsCaptureItem> {
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForMonitor(monitor_handle) }
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
}

impl YUVSource for BGR0YUVConverter {
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

fn take_screenshot(item: &GraphicsCaptureItem) -> Result<()> {
    let item_size = item.Size()?;

    let d3d_device = d3d::create_d3d_device()?;
    let d3d_context = unsafe {
        let mut d3d_context = None;
        d3d_device.GetImmediateContext(&mut d3d_context);
        d3d_context.unwrap()
    };
    let device = d3d::create_direct3d_device(&d3d_device)?;
    println!("device: {:?}", device);
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        item_size,
    )?;
    let session = frame_pool.CreateCaptureSession(item)?;
    println!("session: {:?}", session);

    let (sender, receiver) = channel();

    frame_pool.FrameArrived(
        &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
            move |frame_pool, _| {
                let frame_pool = frame_pool.as_ref().unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                sender.send(frame).unwrap();
                Ok(())
            }
        }),
    )?;

    println!("frame_pool: {:?}", frame_pool);
    session.StartCapture()?;
    use openh264::encoder::{Encoder, EncoderConfig};

    let config = EncoderConfig::new(3840, 2160).max_frame_rate(60.0).enable_skip_frame(true).set_bitrate_bps(8*1024*1024);
    let mut encoder = Encoder::with_config(config).unwrap();
    let mut converter = BGR0YUVConverter::new(3840, 2160);
    let mut output = File::create("output.h264").unwrap();
    
    let mut counter = 0;
    let start = Instant::now();
    while let Ok(frame) = receiver.recv() {
        let timer = Instant::now();
        let (w,h) = (frame.ContentSize()?.Width, frame.ContentSize()?.Height);
        // println!("Got frame {}*{}", frame.ContentSize()?.Width, frame.ContentSize()?.Height);
        //let surface = frame.Surface()?;
        //session.Close()?;
       // println!("frame time {}", frame.SystemRelativeTime()?.Duration);
        let texture = unsafe {
            let source_texture: ID3D11Texture2D =
                d3d::get_d3d_interface_from_object(&frame.Surface()?)?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            source_texture.GetDesc(&mut desc);
            desc.BindFlags = D3D11_BIND_FLAG(0);
            desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);
            desc.Usage = D3D11_USAGE_STAGING;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
            let copy_texture = { d3d_device.CreateTexture2D(&desc, None)? };
            let src: ID3D11Resource = source_texture.cast()?;
            let dst: ID3D11Resource = copy_texture.cast()?;
            d3d_context.CopyResource(&dst, &src);
            copy_texture
        };
        println!("texture time {}", timer.elapsed().as_millis());
        //let texture: ID3D11Texture2D = d3d::get_d3d_interface_from_object(&frame.Surface()?)?;
        unsafe {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc as *mut _);

            let resource: ID3D11Resource = texture.cast()?;
            let mapped = d3d_context.Map(&resource, 0, D3D11_MAP_READ, 0)?;

            // Get a slice of bytes
            let slice: &[u8] = {
                std::slice::from_raw_parts(
                    mapped.pData as *const _,
                    (desc.Height * mapped.RowPitch) as usize,
                )
            };
            println!("pre convert time {}", timer.elapsed().as_millis());
            converter.convert(slice);
            println!("post convert time {}", timer.elapsed().as_millis());

            let data = SSourcePicture {
                iColorFormat: videoFormatI420,
                iStride: [converter.y_stride(), converter.u_stride(), converter.v_stride(), 0],
                pData: [
                    converter.y().as_ptr() as *mut c_uchar,
                    converter.u().as_ptr() as *mut c_uchar,
                    converter.v().as_ptr() as *mut c_uchar,
                    null_mut(),
                ],
                iPicWidth: w,
                iPicHeight: h,
                uiTimeStamp: frame.SystemRelativeTime()?.Duration/10000,
            };
            let mut info = SFrameBSInfo::default();
            //println!("timestamp {:#?}", frame.SystemRelativeTime()?.Duration/10000);
            println!("pre encoding time {}", timer.elapsed().as_millis());
            encoder.raw_api().encode_frame(&data, &mut info);
            println!("post encoding time {}", timer.elapsed().as_millis());
            for l in 0..info.iLayerNum {
                let layer = &info.sLayerInfo[l as usize];

                for n in 0..layer.iNalCount {
                    let mut offset = 0;

                    let slice = unsafe {
                        for nal_idx in 0..n {
                            let size = *layer.pNalLengthInByte.add(nal_idx as usize) as usize;
                            offset += size;
                        }

                        let size = *layer.pNalLengthInByte.add(n as usize) as usize;
                        std::slice::from_raw_parts(layer.pBsBuf.add(offset), size)
                    };

                    output.write_all(slice).expect("TODO: panic message");
                }
            }
/*
            converter.convert(slice);
            let result = encoder.encode(&converter).unwrap();
            let info = result.raw_info();

            output.write_all(&*result.to_vec()).expect("failed to write");
            println!("post write time {}", timer.elapsed().as_millis());
            println!("encoded frame {:#?} ts {} layers {}, size {}",
                     result.frame_type(), info.uiTimeStamp, info.iLayerNum, info.iFrameSizeInBytes);
            for layer in info.sLayerInfo.iter().take(info.iLayerNum as usize) {
                println!("layer ftype {}/{} size {} spid {}, tid {} sequence id {}, quality id {} ",
                         layer.eFrameType, layer.uiLayerType, layer.iNalCount,
                         layer.uiSpatialId, layer.uiTemporalId, layer.iSubSeqId, layer.uiQualityId,
                );
            }*/

            /*
            let bytes_per_pixel = 4;
            //let mut bits = vec![0u8; (desc.Width * desc.Height * bytes_per_pixel) as usize];
            for row in 0..desc.Height {
                let data_begin = (row * (desc.Width * bytes_per_pixel)) as usize;
                let data_end = ((row + 1) * (desc.Width * bytes_per_pixel)) as usize;
                let slice_begin = (row * mapped.RowPitch) as usize;
                let slice_end = slice_begin + (desc.Width * bytes_per_pixel) as usize;
                //out.write_all(&slice[slice_begin..slice_end]).unwrap();
                //bits[data_begin..data_end].copy_from_slice(&slice[slice_begin..slice_end]);
            }*/

            d3d_context.Unmap(&resource, 0);
            println!("finish time {}", timer.elapsed().as_millis());
            ()
            //bits
        };
        // sleep 1s
        //std::thread::sleep(Duration::from_millis(1000));
        counter += 1;
        if counter % 100 == 0 {
            println!("FPS: {}", counter as f64 / start.elapsed().as_secs_f64());
            println!("time elapsed: {:?}", start.elapsed());
        }
        if counter >= 2*60 {
            session.Close();
            frame_pool.Close();
            break;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    use scrap::Display;
    let d = Display::primary().unwrap();
    let (w, h) = (d.width(), d.height());
    println!("{}x{} screen", w, h);
/*
    let child = Command::new("C:\\Users\\Null\\Documents\\Projects\\mira_sharer\\ffplay.exe")
    .args(&[
        "-f", "rawvideo",
        "-pixel_format", "bgr0",
        "-video_size", &format!("{}x{}", w, h),
        "-framerate", "60",
        "-"
    ])
    .stdin(Stdio::piped())
    .spawn()
    .expect("This example requires ffplay.");
    let mut out = child.stdin.unwrap();*/

    /*use scrap::{Capturer};
    use std::io::Write;
    use std::io::ErrorKind::WouldBlock;
    use std::process::{Command, Stdio};

    let mut capturer = Capturer::new(Display::all().unwrap().into_iter().nth(1).unwrap()).unwrap();

    let mut counter = 0;
    let start = Instant::now();
    loop {
        match capturer.frame() {
            Ok(frame) => {
                counter += 1;
                let current = counter;
                if current % 100 == 0 {
                    println!("FPS: {}", current as f64 / start.elapsed().as_secs_f64());
                }
                if counter >= 1000 {
                    break;
                }

                // Write the frame, removing end-of-row padding.
                let stride = frame.len() / h;
                let rowlen = 4 * w;
                for row in frame.chunks(stride) {
                    let row = &row[..rowlen];
                    //out.write_all(row).unwrap();
                }
            }
            Err(ref e) if e.kind() == WouldBlock => {
                // Wait for the frame.
                //std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(_) => {
                // We're done here.
                break;
            }
        }
    }

    return Ok(());*/

    let displays = unsafe {
        let displays = Box::into_raw(Box::new(Vec::<DisplayInfo>::new()));
        EnumDisplayMonitors(
            HDC(0),
            None,
            Some(enum_monitor),
            LPARAM(displays as isize),
        );
        Box::from_raw(displays)
    };
    for display in displays.iter() {
        println!("Display: {} {}", display.display_name, display.handle.0);
    }
    let item = create_capture_item_for_monitor(displays[1].handle)?;
    println!("Item: {:?}", item);
    take_screenshot(&item)?;
    Ok(())
}

// callback function for EnumDisplayMonitors
extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        // get the vector from the param, use leak because this function is not responsible for its lifetime
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<DisplayInfo>));
        let display_info = DisplayInfo::new(monitor).unwrap();
        state.push(display_info);
    }
    true.into()
}
