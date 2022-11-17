pub mod display;
pub mod macos_capture;
mod ffi;
mod frame;
mod config;

pub use macos_capture::MacOSScreenCapture;
pub use display::Display;
