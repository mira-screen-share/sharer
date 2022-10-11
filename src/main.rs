mod d3d;

use std::ffi::c_uchar;
use std::fs::File;
use std::io::Write;
use std::{mem, slice};
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
use x264_sys::{
    X264_CSP_BGRA, x264_nal_t,x264_picture_alloc, x264_picture_t , x264_param_default_preset,
    x264_encoder_open, x264_encoder_close, x264_param_apply_profile, x264_encoder_encode
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

unsafe fn take_screenshot(item: &GraphicsCaptureItem) -> Result<()> {
    let item_size = item.Size()?;
    let (device, d3d_device, d3d_context) = d3d::create_direct3d_devices_and_context()?;
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &d3d_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        item_size,
    )?;
    let session = frame_pool.CreateCaptureSession(item)?;

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

    session.StartCapture()?;

    let mut par = mem::MaybeUninit::uninit();
    x264_param_default_preset(
        par.as_mut_ptr(),
        b"ultrafast\0".as_ptr() as *const i8,
        b"zerolatency\0".as_ptr() as *const i8
    );
    let mut par = par.assume_init();
    par.i_width = 3840;
    par.i_height = 2160;
    par.i_fps_num = 60;
    par.b_annexb = true as i32;

    par.i_csp = X264_CSP_BGRA as i32;
    let mut pic_in : x264_picture_t = mem::MaybeUninit::uninit().assume_init();
    x264_picture_alloc( &mut pic_in, par.i_csp, par.i_width, par.i_height );
    let x = x264_encoder_open(&mut par);

    let mut nal: *mut x264_nal_t = mem::MaybeUninit::uninit().assume_init();
    let mut nal_size = 0;
    let mut pic_out: x264_picture_t = mem::MaybeUninit::uninit().assume_init();

    let mut output = File::create("output.h264").unwrap();

    let mut counter = 0;
    let mut start_relative_time = None;
    let mut encoding_time = Vec::new();
    let start = Instant::now();
    while let Ok(frame) = receiver.recv() {
        let timer = Instant::now();
        let frame_ms = frame.SystemRelativeTime()?.Duration / 10000;
        if start_relative_time.is_none() {
            start_relative_time = Some(frame_ms);
        }
        let texture = unsafe {
            let source_texture: ID3D11Texture2D =
                d3d::get_d3d_interface_from_object(&frame.Surface()?)?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            source_texture.GetDesc(&mut desc);
            desc.BindFlags = D3D11_BIND_FLAG(0);
            desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);
            desc.Usage = D3D11_USAGE_STAGING;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
            let copy_texture = { device.CreateTexture2D(&desc, None)? };
            let src: ID3D11Resource = source_texture.cast()?;
            let dst: ID3D11Resource = copy_texture.cast()?;
            d3d_context.CopyResource(&dst, &src);
            copy_texture
        };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc as *mut _);

        let resource: ID3D11Resource = texture.cast()?;
        let mapped = d3d_context.Map(&resource, 0, D3D11_MAP_READ, 0)?;

        let slice: &[u8] = slice::from_raw_parts(
            mapped.pData as *const _,
            (desc.Height * mapped.RowPitch) as usize,
        );
        pic_in.img.plane=[
            (slice.as_ptr() as *mut u8).add(0),
            null_mut(),
            null_mut(),
            null_mut(),
        ];
        pic_in.i_pts = ((frame_ms - start_relative_time.unwrap()) as f64 / (1.0/60.0*1000.0)).round() as i64;
        let frame_size = x264_encoder_encode(x, &mut nal as *mut *mut _, &mut nal_size, &mut pic_in, &mut pic_out);
        d3d_context.Unmap(&resource, 0);
        output.write_all(slice::from_raw_parts((*nal).p_payload, frame_size as usize)).expect("TODO: panic message");

        encoding_time.push(timer.elapsed().as_millis());
        counter += 1;
        if counter % 100 == 0 {
            let expected_frames = (start.elapsed().as_millis() as f64)/(1.0/60.0*1000.0);
            println!("fps: {}", counter as f64 / start.elapsed().as_secs_f64());
            println!("loss: {} ({}%)", expected_frames - counter as f64, (expected_frames - counter as f64) / expected_frames * 100.0);
            println!("time elapsed: {:?}", start.elapsed());
        }
        if counter >= 3000 {
            session.Close();
            output.flush().expect("");
            frame_pool.Close();
            break;
        }
    }

    println!("encoding time: avg {}, max {}, min {}", encoding_time.iter().sum::<u128>() as f64 / encoding_time.len() as f64, encoding_time.iter().max().unwrap(), encoding_time.iter().min().unwrap());
    x264_encoder_close(x);
    Ok(())
}

fn main() -> Result<()> {
    use scrap::Display;
    let d = Display::primary().unwrap();
    let (w, h) = (d.width(), d.height());
    println!("{}x{} screen", w, h);

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
    unsafe{take_screenshot(&item)?;}
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
