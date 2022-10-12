use crate::display::DisplayInfo;
use crate::result::Result;
use crate::capture::ScreenCapture;
use crate::output::OutputSink;
use crate::encoder::Encoder;

mod d3d;
mod display;
mod result;
mod capture;
mod encoder;
mod output;

fn main() -> Result<()> {
    let displays = DisplayInfo::displays()?;
    for display in displays.iter() {
        println!("Display: {} {}x{}", display.display_name, display.resolution.0, display.resolution.1);
    }
    let display = displays.iter().nth(1).unwrap();
    let item = display.create_capture_item_for_monitor()?;
    let mut capture = capture::WGCScreenCapture::new(&item)?;
    let mut encoder = encoder::X264Encoder::new(display.resolution.0, display.resolution.1);
    let mut output = output::FileOutput::new("output.h264");
    capture.capture(&mut encoder, &mut output)?;
    Ok(())
}
