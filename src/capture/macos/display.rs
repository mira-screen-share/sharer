use apple_sys::ScreenCaptureKit::{
    INSDictionary, INSNumber, INSScreen, ISCDisplay, NSDictionary, NSNumber, NSScreen, NSString,
    NSString_NSStringDeprecated, SCDisplay,
};

use crate::capture::macos::ffi::{from_nsarray, from_nsstring, FromNSArray};
use crate::capture::DisplayInfo;

#[derive(Clone, Debug)]
pub struct Display {
    pub sc_display: SCDisplay,
    scale_factor: usize,
    name: String,
}

unsafe impl Send for Display {}

impl Display {
    pub fn new(sc_display: SCDisplay) -> Self {
        let ns_screen = unsafe { try_get_ns_screen(sc_display) };
        let scale_factor = ns_screen
            .as_ref()
            .map(|screen| unsafe { screen.backingScaleFactor() as usize })
            .unwrap_or(2);
        Self {
            sc_display,
            scale_factor,
            name: unsafe { get_name(sc_display, scale_factor, ns_screen) },
        }
    }
}

impl ToString for Display {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}

impl PartialEq<Self> for Display {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.sc_display.displayID() == other.sc_display.displayID() }
    }
}

impl Eq for Display {}

impl DisplayInfo for Display {
    fn resolution(&self) -> (u32, u32) {
        unsafe {
            (
                self.sc_display.width() as u32 * self.scale_factor as u32,
                self.sc_display.height() as u32 * self.scale_factor as u32,
            )
        }
    }

    fn dpi_conversion_factor(&self) -> f64 {
        self.scale_factor as f64
    }
}

unsafe fn try_get_ns_screen(display: SCDisplay) -> Option<NSScreen> {
    from_nsarray!(NSScreen, NSScreen::screens())
        .iter()
        .find_map(|screen| {
            let screen_dictionary = screen.deviceDescription();
            if screen_dictionary.0.is_null() {
                return None;
            }
            let screen_id = NSNumber(
                <NSDictionary as INSDictionary<NSString, NSNumber>>::objectForKey_(
                    &screen_dictionary,
                    NSString::alloc().initWithCString_(b"NSScreenNumber\0".as_ptr() as *const _),
                ),
            );
            if screen_id.unsignedIntValue() == display.displayID() {
                Some(screen.clone())
            } else {
                None
            }
        })
}

unsafe fn get_name(display: SCDisplay, scale_factor: usize, ns_screen: Option<NSScreen>) -> String {
    let id = display.displayID();
    let width = display.width() as usize * scale_factor;
    let height = display.height() as usize * scale_factor;
    let name = ns_screen
        .map(|screen| from_nsstring!(screen.localizedName()).to_string())
        .unwrap_or(format!("Display {}", id));

    format!("{} ({} x {})", name, width, height)
}
