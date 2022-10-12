use crate::{Encoder, OutputSink, Result};

pub trait ScreenCapture {
    fn capture(&mut self, encoder: &mut dyn Encoder, output: &mut dyn OutputSink) -> Result<()>;
}

mod wgc_capture;

pub use wgc_capture::WGCScreenCapture;