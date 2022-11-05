use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use crate::result::Result;

#[derive(Clone)]
pub struct DisplayInfo {
    pub handle: HMONITOR,
    pub display_name: String,
    pub resolution: (u32, u32),
}

impl DisplayInfo {
    pub fn displays() -> Result<Vec<Self>> {
        unsafe {
            let displays = Box::into_raw(Box::new(Vec::<DisplayInfo>::new()));
            EnumDisplayMonitors(
                HDC(0),
                None,
                Some(enum_monitor),
                LPARAM(displays as isize),
            );
            Ok(*Box::from_raw(displays))
        }
    }

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
            resolution: ((info.monitorInfo.rcMonitor.right - info.monitorInfo.rcMonitor.left) as u32,
                         (info.monitorInfo.rcMonitor.bottom - info.monitorInfo.rcMonitor.top) as u32),
        })
    }

    pub fn create_capture_item_for_monitor(&self) -> Result<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        Ok(unsafe { interop.CreateForMonitor(self.handle) }?)
    }
}

// callback function for EnumDisplayMonitors
extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        // get the vector from the param, use leak because this function is not responsible for its lifetime
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<DisplayInfo>));
        state.push(DisplayInfo::new(monitor).unwrap());
    }
    true.into()
}
