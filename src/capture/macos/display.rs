use failure::format_err;

use crate::capture::DisplayInfo;
use crate::result::Result;

use super::ffi::*;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(C)]
pub struct Display(u32);

impl Display {
    pub fn online() -> Result<Vec<Display>> {
        unsafe {
            let mut displays = Vec::with_capacity(16);
            let mut len: u32 = 0;

            match CGGetOnlineDisplayList(16, displays.as_mut_ptr(), &mut len) {
                CGError::Success => (),
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
        unsafe { CGDisplayModeGetPixelHeight(CGDisplayCopyDisplayMode(self.id())) }
    }
}

impl DisplayInfo for Display {
    fn resolution(&self) -> (u32, u32) {
        (self.width() as u32, self.height() as u32)
    }
}
