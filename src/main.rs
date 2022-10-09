mod d3d;
use std::sync::mpsc::channel;
use std::time::Instant;
use windows::core::{IInspectable, Result};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Foundation::TypedEventHandler;
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

    let mut counter = 0;
    let start = Instant::now();
    while let Ok(frame) = receiver.recv() {
        // println!("Got frame {}*{}", frame.ContentSize()?.Width, frame.ContentSize()?.Height);
        //let surface = frame.Surface()?;
        //session.Close()?;
        println!("frame time {}", frame.SystemRelativeTime()?.Duration);
        counter += 1;
        if counter >= 1000 {
            println!("FPS: {}", counter as f64 / start.elapsed().as_secs_f64());
            session.Close();
            break;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
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
    let item = create_capture_item_for_monitor(displays[0].handle)?;
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
