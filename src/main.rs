use crate::display::DisplayInfo;
use crate::result::Result;
use crate::capture::ScreenCapture;
use crate::output::{OutputSink, WebRTCOutput};
use crate::encoder::Encoder;
use clap::Parser;

mod d3d;
mod display;
mod result;
mod capture;
mod encoder;
mod output;
mod signaller;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Display index
    #[arg(short, long, default_value = "0")]
    display: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let displays = DisplayInfo::displays()?;
    for (i, display) in displays.iter().enumerate() {
        println!(
            "display: {} {}x{} {}",
            display.display_name,
            display.resolution.0,
            display.resolution.1,
            if i==args.display { "(selected)" } else { "" },
        );
    }
    let display = displays.iter().nth(args.display).unwrap();
    let item = display.create_capture_item_for_monitor()?;
    let mut capture = capture::WGCScreenCapture::new(&item)?;
    let mut encoder = encoder::X264Encoder::new(display.resolution.0, display.resolution.1);
    let mut output = output::FileOutput::new("output.h264");
    let config = WebRTCOutput::make_config(
        &vec![String::from("stun:stun.l.google.com:19302")]
    );
    capture.capture(&mut encoder, &mut output)?;
    Ok(())
}
