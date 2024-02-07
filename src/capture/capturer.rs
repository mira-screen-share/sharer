use std::sync::Arc;

use clap::Parser;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::auth::{ComplexAuthenticator, PasswordAuthenticator, ViewerIdentifier, ViewerManager};
#[allow(unused_imports)]
use crate::capture::audio::AudioCapture;
use crate::capture::display::DisplaySelector;
use crate::capture::{ScreenCapture, ScreenCaptureImpl};
use crate::config::Config;
use crate::encoder;
use crate::inputs::InputHandler;
use crate::output::{FileOutput, OutputSink, WebRTCOutput};
use crate::performance_profiler::PerformanceProfiler;
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
    #[arg(short, long)]
    pub(crate) config: Option<String>,
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
    capture: Arc<Mutex<ScreenCaptureImpl>>,
    room_password: String,
    viewer_manager: Arc<ViewerManager>,
}

impl Capturer {
    pub fn new(args: Args, config: Config, notify_update: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            args,
            config: config.clone(),
            shutdown_token_opt: None,
            signaller: Arc::new(Mutex::new(None)),
            notify_update: notify_update.clone(),
            capture: Arc::new(Mutex::new(ScreenCaptureImpl::new(config.clone()).unwrap())),
            room_password: "".to_string(),
            viewer_manager: Arc::new(ViewerManager::new(notify_update)),
        }
    }

    pub fn get_viewer_manager(&self) -> Arc<ViewerManager> {
        self.viewer_manager.clone()
    }

    pub async fn kick_viewer(&self, id: ViewerIdentifier) -> () {
        match self.signaller.try_lock() {
            Ok(signaller) => {
                if let Some(signaller) = signaller.as_ref() {
                    signaller.kick_viewer(id.uuid).await;
                }
            }
            Err(e) => {
                error!("Failed to lock signaller while kicking viewer: {}", e);
            }
        }
    }

    pub fn run(&mut self) {
        let args = self.args.clone();
        let config = self.config.clone();

        let shutdown_token = CancellationToken::new();
        self.shutdown_token_opt.replace(shutdown_token.clone());
        self.capture(args, config, shutdown_token.clone());
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
            "{}?room={}&pwd={}&signaller={}",
            self.config.viewer_url,
            self.get_room_id().unwrap_or_default(),
            self.room_password,
            self.config.signaller_url
        ))
    }

    pub fn get_room_id(&self) -> Option<String> {
        match self.signaller.try_lock() {
            Ok(signaller) => signaller.as_ref().and_then(|s| s.get_room_id()),
            Err(e) => {
                error!("Failed to get room id: {}", e);
                None
            }
        }
    }

    pub fn get_room_password(&self) -> Option<String> {
        if !self.room_password.is_empty() {
            Some(self.room_password.clone())
        } else {
            None
        }
    }

    pub fn available_displays(&self) -> Vec<<ScreenCaptureImpl as DisplaySelector>::Display> {
        match self.capture.try_lock() {
            Ok(mut capturer) => capturer.available_displays().unwrap(),
            Err(e) => {
                error!("Failed to get available displays: {}", e);
                Vec::new()
            }
        }
    }

    pub fn selected_display(&self) -> Option<<ScreenCaptureImpl as DisplaySelector>::Display> {
        match self.capture.try_lock() {
            Ok(capturer) => capturer.selected_display().unwrap(),
            Err(e) => {
                error!("Failed to get selected display: {}", e);
                None
            }
        }
    }

    pub fn select_display(&self, display: <ScreenCaptureImpl as DisplaySelector>::Display) {
        match self.capture.try_lock() {
            Ok(mut capturer) => capturer.select_display(&display).unwrap(),
            Err(e) => {
                error!("Failed to select display: {}", e);
            }
        }
    }

    async fn handle_left_viewers(
        signaller: Arc<dyn Signaller + Send + Sync>,
        view_manager: Arc<ViewerManager>,
        shutdown_token: CancellationToken,
    ) {
        loop {
            if shutdown_token.is_cancelled() {
                break;
            }
            if let Some(left_viewer) = signaller
                .blocking_wait_leave_message(shutdown_token.clone())
                .await
            {
                info!("Viewer left: {}", left_viewer);
                view_manager.viewer_left(&left_viewer).await;
            }
        }
    }

    fn capture(&mut self, args: Args, config: Config, shutdown_token: CancellationToken) {
        let profiler = PerformanceProfiler::new(args.profiler, config.max_fps);
        let signaller_opt = self.signaller.clone();
        let notify_update = self.notify_update.clone();
        let capture = self.capture.clone();

        let password_auth = Arc::new(PasswordAuthenticator::random().unwrap());
        let viewer_manager = self.viewer_manager.clone();
        self.room_password = password_auth.password();

        tokio::spawn(async move {
            {
                let mut capture = capture.lock().await;
                let signaller_url = config.signaller_url.clone();
                let signaller = Arc::new(
                    WebSocketSignaller::new(&signaller_url, notify_update.clone())
                        .await
                        .unwrap(),
                );

                tokio::spawn(Self::handle_left_viewers(
                    signaller.clone(),
                    viewer_manager.clone(),
                    shutdown_token.clone(),
                ));

                signaller_opt.lock().await.replace(signaller.clone());
                let resolution = capture.display().resolution();
                let mut encoder =
                    encoder::FfmpegEncoder::new(resolution.0, resolution.1, &config.encoder);
                let input_handler = Arc::new(InputHandler::new(
                    args.disable_control,
                    capture.display().dpi_conversion_factor(),
                ));

                let output: Arc<Mutex<dyn OutputSink + Send>> = if let Some(path) = args.file {
                    Arc::new(Mutex::new(FileOutput::new(&path)))
                } else {
                    let webrtc = WebRTCOutput::new(
                        signaller,
                        Arc::new(ComplexAuthenticator::new(vec![
                            password_auth,
                            viewer_manager.clone(),
                        ])),
                        &mut encoder.force_idr,
                        input_handler.clone(),
                        &config,
                    )
                    .await
                    .unwrap();
                    viewer_manager.set_webrtc_output(webrtc.clone()).await;
                    webrtc
                };

                #[cfg(target_os = "windows")]
                AudioCapture::capture(output.clone(), shutdown_token.clone()).unwrap();

                capture
                    .start_capture(encoder, output, profiler, shutdown_token.clone())
                    .await
                    .unwrap();
            }
            notify_update(); // Update when capture starts

            shutdown_token.cancelled().await;

            // Cleanup
            capture.lock().await.stop_capture().await.unwrap();
            if let Some(signaller) = signaller_opt.lock().await.take() {
                signaller.leave().await;
                signaller.close().await;
            }

            viewer_manager.clear().await;

            notify_update(); // Update when capture stops
        });
    }
}
