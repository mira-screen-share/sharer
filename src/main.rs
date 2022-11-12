use crate::capture::display::Display;
use crate::capture::ScreenCapture;
use crate::output::{FileOutput, OutputSink, WebRTCOutput};
use crate::performance_profiler::PerformanceProfiler;
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

const DEFAULT_VIEWER_URL: &str = "https://mirashare.app/";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The index of the display to capture
    #[arg(short, long, default_value = "0")]
    display: usize,
    /// signaller url
    #[arg(short, long, default_value = "wss://ws.mirashare.app")]
    url: String,
    /// enable profiler output
    #[arg(long, default_value = "false")]
    profiler: bool,
    /// if provided, will stream to file instead of webrtc
    #[arg(long)]
    file: Option<String>,
    /// max fps
    #[arg(long, default_value = "30")]
    max_fps: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = Args::parse();

    let display = DisplayInfo::displays()?[args.display].select()?;
    let profiler = PerformanceProfiler::new(args.profiler, args.max_fps);
    let mut capture = capture::WGCScreenCapture::new(&display)?;
    let mut encoder = encoder::FfmpegEncoder::new(display.resolution().0, display.resolution().1);
    let input_handler = Arc::new(inputs::InputHandler::new());

    let my_uuid = uuid::Uuid::new_v4().to_string();
    info!(
        "Invite link: {}?room={}&signaller={}",
        DEFAULT_VIEWER_URL, my_uuid, args.url
    );

    let output: Box<dyn OutputSink + Send> = if let Some(path) = args.file {
        Box::new(FileOutput::new(&path))
    } else {
        WebRTCOutput::new(
            WebRTCOutput::make_config(),
            Box::new(signaller::WebSocketSignaller::new(&args.url, my_uuid).await?),
            &mut encoder.force_idr,
            input_handler.clone(),
        )
        .await?
    };

    capture
        .capture(encoder, output, profiler, args.max_fps)
        .await?;

    Ok(())
}
