use crate::capture::ScreenCapture;
use crate::encoder::Encoder;
use crate::output::{FileOutput, OutputSink, WebRTCOutput};
use crate::result::Result;
use capture::display::DisplayInfo;
use clap::Parser;

#[macro_use]
extern crate log;

mod capture;
mod encoder;
mod output;
mod performance_profiler;
mod result;
mod signaller;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Display index
    #[arg(short, long, default_value = "0")]
    display: usize,
    /// signaller url
    #[arg(short, long, default_value = "ws://localhost:8080")]
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

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
    let webrtc_output = WebRTCOutput::new(
        WebRTCOutput::make_config(&["stun:stun.l.google.com:19302".into()]),
        Box::new(signaller::WebSocketSignaller::new(&args.url).await?),
    )
    .await?;
    //let file_output = Box::new(FileOutput::new("output.h264"));
    capture.capture(encoder, webrtc_output).await?;
    Ok(())
}
