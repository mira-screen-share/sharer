use crate::capture::ScreenCapture;
use crate::encoder::Encoder;
use crate::output::{FileOutput, OutputSink, WebRTCOutput};
use crate::result::Result;
use capture::display::DisplayInfo;
use clap::Parser;
use std::sync::Arc;

#[macro_use]
extern crate log;

mod capture;
mod encoder;
mod inputs;
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
    let mut encoder = Box::new(encoder::X264Encoder::new(
        display.resolution.0,
        display.resolution.1,
    ));
    let input_handler = Arc::new(inputs::InputHandler::new());

    let my_uuid = "00000000-0000-0000-0000-000000000000".to_string(); //uuid::Uuid::new_v4().to_string();
    info!("Room uuid: {}", my_uuid);

    let webrtc_output = WebRTCOutput::new(
        WebRTCOutput::make_config(&["stun:stun.l.google.com:19302".into()]),
        Box::new(signaller::WebSocketSignaller::new(&args.url, my_uuid).await?),
        &mut encoder.force_idr,
        input_handler.clone(),
    )
    .await?;
    //let file_output = Box::new(FileOutput::new("output.h264"));
    capture.capture(encoder, webrtc_output).await?;
    Ok(())
}
