mod config;
pub mod display;
mod ffi;
pub mod macos_capture;
pub mod macos_sc_capture;

pub use display::Display;
pub use macos_capture::MacOSScreenCapture;
