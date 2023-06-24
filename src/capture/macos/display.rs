use anyhow::format_err;
use apple_sys::CoreMedia::{
    CGDisplayCopyDisplayMode, CGDisplayModeGetPixelHeight, CGDisplayModeGetPixelWidth,
    CGDisplayPixelsHigh, CGError_kCGErrorSuccess, CGGetOnlineDisplayList,
};

use crate::capture::DisplayInfo;
use crate::result::Result;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(C)]
pub struct Display(u32);

// TODO
impl Display {
    pub fn online() -> Result<Vec<Display>> {
        unsafe {
            let mut displays = Vec::with_capacity(16);
            let mut len: u32 = 0;

            #[allow(non_upper_case_globals)]
            match CGGetOnlineDisplayList(16, displays.as_mut_ptr(), &mut len) {
                CGError_kCGErrorSuccess => (),
                x => return Err(format_err!("CGGetOnlineDisplayList failed: {:?}", x)),
            }

            displays.set_len(len as usize);

            Ok(displays.iter().map(|it| Display(*it)).collect())
        }
    }

    pub fn select(self) -> Result<Self> {
        Ok(self)
    }

    pub fn id(self) -> u32 {
        self.0
    }

    pub fn width(self) -> usize {
        unsafe { CGDisplayModeGetPixelWidth(CGDisplayCopyDisplayMode(self.id())) }
    }

    pub fn height(self) -> usize {
        // unsafe { CGDisplayPixelsHigh(self.id()) }
        unsafe { CGDisplayModeGetPixelHeight(CGDisplayCopyDisplayMode(self.id())) }
    }
}

impl DisplayInfo for Display {
    fn resolution(&self) -> (u32, u32) {
        (self.width() as u32, self.height() as u32)
    }
    fn dpi_conversion_factor(&self) -> f64 {
        self.height() as f64 / unsafe { CGDisplayPixelsHigh(self.id()) } as f64
    }
}
