use std::sync::Arc;

use clap::Parser;
use cpal::traits::StreamTrait;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::capture::{AudioCapture, Display, DisplayInfo, ScreenCapture, ScreenCaptureImpl};
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
    invite_link_opt: Option<String>,
    room_id_opt: Option<String>,
    signaller: Arc<dyn Signaller + Send + Sync>,
}

impl Capturer {
    pub async fn new(args: Args, config: Config) -> Self {
        let signaller_url = config.signaller_url.clone();
        Self {
            args,
            config,
            shutdown_token_opt: None,
            invite_link_opt: None,
            room_id_opt: None,
            signaller: Arc::new(WebSocketSignaller::new(&signaller_url).await.unwrap()),
        }
    }

    pub fn run(&mut self) -> () {
        let args = self.args.clone();
        let config = self.config.clone();

        let shutdown_token = CancellationToken::new();
        self.shutdown_token_opt = Some(shutdown_token.clone());

        let sharer_uuid = uuid::Uuid::new_v4().to_string();
        self.invite_link_opt = Some(format!(
            "{}?room={}&signaller={}",
            config.viewer_url, sharer_uuid, config.signaller_url
        ));
        self.room_id_opt = Some(sharer_uuid.clone());

        let signaller_clone = self.signaller.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = start_capture(args, config, sharer_uuid, signaller_clone) => {}
                _ = shutdown_token.cancelled() => {}
            }
        });
    }

    pub fn shutdown(&mut self) {
        if let Some(shutdown_token) = self.shutdown_token_opt.take() {
            shutdown_token.cancel();
            self.invite_link_opt = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.shutdown_token_opt.is_some()
    }

    pub fn get_invite_link(&self) -> Option<String> {
        self.invite_link_opt.clone()
    }

    pub fn get_room_id(&self) -> Option<String> {
        self.room_id_opt.clone()
    }
}

async fn start_capture(
    args: Args,
    config: Config,
    sharer_uuid: String,
    signaller: Arc<dyn Signaller + Send + Sync>,
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
    info!(
        "Invite link: {}?room={}&signaller={}",
        config.viewer_url, sharer_uuid, config.signaller_url
    );
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

    // need to outlive capture.capture, i.e. end of this function
    let _capturer = AudioCapture::capture(output.clone())?;
    capture.capture(encoder, output, profiler).await?;
    Ok(())
}
