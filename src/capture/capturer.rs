use std::sync::mpsc::Sender;
use std::sync::{Arc, Barrier};

use clap::Parser;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::capture::audio::AudioCapture;
use crate::capture::{Display, DisplayInfo, ScreenCapture, ScreenCaptureImpl};
use crate::config::Config;
use crate::encoder;
use crate::inputs::InputHandler;
use crate::output::{FileOutput, OutputSink, WebRTCOutput};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::signaller::{Signaller, WebSocketSignaller};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The index of the display to capture
    #[arg(short, long, default_value = "0")]
    pub(crate) display: usize,
    /// Enable profiler output
    #[arg(long, default_value = "false")]
    profiler: bool,
    /// If provided, will stream to file instead of webrtc
    #[arg(long)]
    file: Option<String>,
    /// Config file path
    #[arg(short, long, default_value = "config.toml")]
    pub(crate) config: String,
    /// Disable remote control
    #[arg(long, default_value = "false")]
    disable_control: bool,
}

pub struct Capturer {
    pub args: Args,
    pub config: Config,
    shutdown_token_opt: Option<CancellationToken>,
    signaller: Arc<Mutex<Option<Arc<dyn Signaller + Send + Sync>>>>,
    notify_update: Arc<dyn Fn() + Send + Sync>,
}

impl Capturer {
    pub fn new(args: Args, config: Config, notify_update: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            args,
            config,
            shutdown_token_opt: None,
            signaller: Arc::new(Mutex::new(None)),
            notify_update,
        }
    }

    pub fn run(&mut self) {
        let args = self.args.clone();
        let config = self.config.clone();

        let shutdown_token = CancellationToken::new();
        self.shutdown_token_opt = Some(shutdown_token.clone());

        let signaller_opt = self.signaller.clone();
        let notify_update = self.notify_update.clone();
        tokio::spawn(async move {
            let signaller_url = config.signaller_url.clone();
            let signaller = Arc::new(
                WebSocketSignaller::new(&signaller_url, notify_update)
                    .await
                    .unwrap(),
            );
            signaller_opt.lock().await.replace(signaller.clone());

            tokio::select! {
                _ = start_capture(args, config, signaller, shutdown_token.clone()) => {}
                _ = shutdown_token.cancelled() => {}
            }
        });
    }

    pub fn shutdown(&mut self) {
        if let Some(shutdown_token) = self.shutdown_token_opt.take() {
            shutdown_token.cancel();
        }
    }

    pub fn is_running(&self) -> bool {
        self.shutdown_token_opt.is_some()
    }

    pub fn get_invite_link(&self) -> Option<String> {
        Some(format!(
            "{}?room={}&signaller={}",
            self.config.viewer_url,
            self.get_room_id().unwrap_or_default(),
            self.config.signaller_url
        ))
    }

    pub fn get_room_id(&self) -> Option<String> {
        self.signaller
            .clone()
            .try_lock()
            .unwrap() // TODO:Fix
            .as_ref()
            .map_or(None, |s| s.get_room_id())
    }
}

async fn start_capture(
    args: Args,
    config: Config,
    signaller: Arc<dyn Signaller + Send + Sync>,
    shutdown_token: CancellationToken,
) -> Result<()> {
    let display = Display::online().unwrap()[args.display].select()?;
    let dpi_conversion_factor = display.dpi_conversion_factor();
    let profiler = PerformanceProfiler::new(args.profiler, config.max_fps);
    let resolution = display.resolution();
    let mut capture = ScreenCaptureImpl::new(display, &config)?;
    let mut encoder = encoder::FfmpegEncoder::new(resolution.0, resolution.1, &config.encoder);
    let input_handler = Arc::new(InputHandler::new(
        args.disable_control,
        dpi_conversion_factor,
    ));

    info!("Resolution: {:?}", resolution);
    let output: Arc<Mutex<dyn OutputSink + Send>> = if let Some(path) = args.file {
        Arc::new(Mutex::new(FileOutput::new(&path)))
    } else {
        WebRTCOutput::new(
            signaller,
            &mut encoder.force_idr,
            input_handler.clone(),
            &config,
        )
        .await?
    };

    #[cfg(target_os = "windows")]
    AudioCapture::capture(output.clone(), shutdown_token)?;

    capture.capture(encoder, output, profiler).await?;
    Ok(())
}
