use std::fs::File;
use std::io::Write;
use crate::Result;

pub trait OutputSink {
    fn write(&mut self, input: &[u8]) -> Result<()>;
}

mod file_output;
mod webrtc_output;

pub use file_output::FileOutput;
pub use webrtc_output::WebRTCOutput;
