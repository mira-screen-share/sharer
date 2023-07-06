use crate::capture::DisplayInfo;
use crate::result::Result;
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoA, HDC, HMONITOR, MONITORINFO, MONITORINFOEXA,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Display {
    pub handle: HMONITOR,
}

impl Display {
    pub fn online() -> Result<Vec<Self>> {
        unsafe {
            let displays = Box::into_raw(Box::default());
            EnumDisplayMonitors(HDC(0), None, Some(enum_monitor), LPARAM(displays as isize));
            Ok(*Box::from_raw(displays))
        }
    }

    pub fn new(handle: HMONITOR) -> Result<Self> {
        Ok(Self { handle })
    }

    pub fn select(&self) -> Result<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        Ok(unsafe { interop.CreateForMonitor(self.handle) }?)
    }
}

impl ToString for Display {
    fn to_string(&self) -> String {
        let mut info = MONITORINFOEXA {
            monitorInfo: MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFOEXA>() as u32,
                ..Default::default()
            },
            szDevice: [0; 32],
        };
        let ptr = &mut info as *mut _ as *mut MONITORINFO;
        unsafe {
            GetMonitorInfoA(self.handle, ptr);
        }
        let name = unsafe { std::ffi::CStr::from_ptr(info.szDevice.as_ptr() as _) };
        let name = name.to_str().unwrap().to_string();
        let width = info.monitorInfo.rcMonitor.right - info.monitorInfo.rcMonitor.left;
        let height = info.monitorInfo.rcMonitor.bottom - info.monitorInfo.rcMonitor.top;

        format!("{} ({} x {})", name, width, height)
    }
}

// callback function for EnumDisplayMonitors
extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        // get the vector from the param, use leak because this function is not responsible for its lifetime
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<Display>));
        state.push(Display::new(monitor).unwrap());
    }
    true.into()
}

impl DisplayInfo for GraphicsCaptureItem {
    fn resolution(&self) -> (u32, u32) {
        (
            self.Size().unwrap().Width as u32,
            self.Size().unwrap().Height as u32,
        )
    }
    fn dpi_conversion_factor(&self) -> f64 {
        1.0
    }
}
