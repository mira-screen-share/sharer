use crate::capture::DisplayInfo;
use std::mem;

use super::ffi::*;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(C)]
pub struct Display(u32);

impl Display {
    pub fn primary() -> Display {
        Display(unsafe { CGMainDisplayID() })
    }

    pub fn online() -> Result<Vec<Display>, CGError> {
        unsafe {
            let mut arr: [u32; 16] = mem::uninitialized();
            let mut len: u32 = 0;

            match CGGetOnlineDisplayList(16, arr.as_mut_ptr(), &mut len) {
                CGError::Success => (),
                x => return Err(x),
            }

            let mut res = Vec::with_capacity(16);
            for i in 0..len as usize {
                res.push(Display(*arr.get_unchecked(i)));
            }
            Ok(res)
        }
    }

    pub fn select(self) -> Result<Self> {
        Ok(self)
    }

    pub fn id(self) -> u32 {
        self.0
    }

    pub fn width(self) -> usize {
        unsafe { CGDisplayPixelsWide(self.0) }
    }

    pub fn height(self) -> usize {
        unsafe { CGDisplayPixelsHigh(self.0) }
    }

    pub fn is_builtin(self) -> bool {
        unsafe { CGDisplayIsBuiltin(self.0) != 0 }
    }

    pub fn is_primary(self) -> bool {
        unsafe { CGDisplayIsMain(self.0) != 0 }
    }

    pub fn is_active(self) -> bool {
        unsafe { CGDisplayIsActive(self.0) != 0 }
    }

    pub fn is_online(self) -> bool {
        unsafe { CGDisplayIsOnline(self.0) != 0 }
    }
}

impl DisplayInfo for Display {
    fn resolution(&self) -> (u32, u32) {
        (self.width() as u32, self.height() as u32)
    }
}
