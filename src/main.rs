use crate::capture::ScreenCapture;
use crate::display::DisplayInfo;
use crate::encoder::Encoder;
use crate::output::{OutputSink, WebRTCOutput};
use crate::result::Result;
use clap::Parser;
use log::LevelFilter;

#[macro_use]
extern crate log;

mod capture;
mod d3d;
mod display;
mod encoder;
mod output;
mod performance_profiler;
mod result;
mod signaller;
mod yuv_converter;

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
            if i == args.display { "(selected)" } else { "" },
        );
    }
    let display = displays.iter().nth(args.display).unwrap();
    let item = display.create_capture_item_for_monitor()?;
    let mut capture = capture::WGCScreenCapture::new(&item)?;
    let encoder = Box::new(encoder::X264Encoder::new(
        display.resolution.0,
        display.resolution.1,
    ));
    let config = WebRTCOutput::make_config(&[String::from("stun:stun.l.google.com:19302")]);
    let mut signaller = signaller::WebSocketSignaller::new(&args.url).await?;
    let mut webrtc_output = Box::new(WebRTCOutput::new(config, &mut signaller).await?);
    //let file_output = Box::new(FileOutput::new("output.h264"));
    capture.capture(encoder, webrtc_output).await?;
    Ok(())
}
