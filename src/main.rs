use crate::display::DisplayInfo;
use crate::result::Result;
use crate::capture::ScreenCapture;
use crate::output::{FileOutput, OutputSink, WebRTCOutput};
use crate::encoder::Encoder;
use clap::Parser;
use log::LevelFilter;

#[macro_use]
extern crate log;

mod d3d;
mod display;
mod result;
mod capture;
mod encoder;
mod output;
mod signaller;
mod yuv_converter;
mod performance_profiler;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Display index
    #[arg(short, long, default_value = "0")]
    display: usize,
    /// signaller url
    #[arg(short, long, default_value = "ws://192.168.0.80:8443")]
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Debug)
        .parse_default_env()
        .init();
    info!("starting up");

    let args = Args::parse();

    let displays = DisplayInfo::displays()?;
    for (i, display) in displays.iter().enumerate() {
        info!(
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
    let _config = WebRTCOutput::make_config(
        &[String::from("stun:stun.l.google.com:19302")]
    );
    //let mut signaller = signaller::WebSocketSignaller::new(&args.url).await?;
    //let mut webrtc_output = WebRTCOutput::new(config, &mut signaller).await?;
    let mut file_output = FileOutput::new("output.h264");
    capture.capture(&mut encoder, &mut file_output)?;
    Ok(())
}
