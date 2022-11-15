use crate::capture::display::Display;
use crate::capture::ScreenCapture;
use crate::output::{FileOutput, OutputSink, WebRTCOutput};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use capture::display::DisplayInfo;
use clap::Parser;
use std::path::Path;
use std::sync::Arc;

#[macro_use]
extern crate log;

mod capture;
mod config;
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
    /// Enable profiler output
    #[arg(long, default_value = "false")]
    profiler: bool,
    /// If provided, will stream to file instead of webrtc
    #[arg(long)]
    file: Option<String>,
    /// Config file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = Args::parse();
    let config = config::load(Path::new(&args.config))?;

    let display = DisplayInfo::displays()?[args.display].select()?;
    let profiler = PerformanceProfiler::new(args.profiler, config.max_fps);
    let resolution = display.resolution();
    let mut capture = capture::WGCScreenCapture::new(display, &config)?;
    let mut encoder = encoder::FfmpegEncoder::new(resolution.0, resolution.1, &config.encoder);
    let input_handler = Arc::new(inputs::InputHandler::new());
    let my_uuid = uuid::Uuid::new_v4().to_string();

    info!(
        "Invite link: {}?room={}&signaller={}",
        config.viewer_url, my_uuid, config.signaller_url
    );

    let output: Box<dyn OutputSink + Send> = if let Some(path) = args.file {
        Box::new(FileOutput::new(&path))
    } else {
        WebRTCOutput::new(
            Box::new(signaller::WebSocketSignaller::new(&config.signaller_url, my_uuid).await?),
            &mut encoder.force_idr,
            input_handler.clone(),
            &config,
        )
        .await?
    };

    capture.capture(encoder, output, profiler).await?;

    Ok(())
}
