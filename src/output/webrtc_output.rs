use crate::inputs::InputHandler;
use async_trait::async_trait;

use crate::config::Config;
use bytes::Bytes;
use log::info;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264};
use webrtc::api::APIBuilder;
use webrtc::interceptor::registry::Registry;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

use crate::output::WebRTCPeer;
use crate::signaller::Signaller;
use crate::OutputSink;
use crate::Result;

pub struct WebRTCOutput {
    api: Arc<webrtc::api::API>,
    peers: Arc<Mutex<Vec<WebRTCPeer>>>,
    video_track: Arc<TrackLocalStaticSample>,
    frame_rate: u32,
}

impl WebRTCOutput {
    fn make_config(config: &Config) -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: config
                .ice_servers
                .clone()
                .into_iter()
                .map(|s| s.into())
                .collect(),
            ..Default::default()
        }
    }

    pub async fn new(
        mut signaller: Box<dyn Signaller>,
        encoder_force_idr: &mut Arc<AtomicBool>,
        input_handler: Arc<InputHandler>,
        config: &Config,
    ) -> Result<Box<WebRTCOutput>> {
        info!("Initializing WebRTC");
        // Create a MediaEngine object to configure the supported codec
        let mut m = MediaEngine::default();

        m.register_default_codecs()?;

        // Create a InterceptorRegistry. This is the user configurable RTP/RTCP Pipeline.
        // This provides NACKs, RTCP Reports and other features. If you use `webrtc.NewPeerConnection`
        // this is enabled by default. If you are manually managing You MUST create a InterceptorRegistry
        // for each PeerConnection.
        let mut registry = Registry::new();

        // Use the default set of Interceptors
        registry = register_default_interceptors(registry, &mut m)?;

        // Create a video track
        let video_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                ..Default::default()
            },
            "video".to_owned(),
            "screen".to_owned(),
        ));

        let output = Box::new(Self {
            api: Arc::new(
                APIBuilder::new()
                    .with_media_engine(m)
                    .with_interceptor_registry(registry)
                    .build(),
            ),
            peers: Arc::new(Mutex::new(Vec::new())),
            video_track: video_track.clone(),
            frame_rate: config.max_fps,
        });

        let api_clone = output.api.clone();
        let peers_clone = output.peers.clone();
        let encoder_force_idr = encoder_force_idr.clone();
        let video_track_clone = video_track.clone();
        let webrtc_config = Self::make_config(config);
        signaller.start().await;
        tokio::spawn(async move {
            while let Some(peer) = signaller.accept_peer().await {
                let api_clone = api_clone.clone();
                let peers_clone = peers_clone.clone();
                let encoder_force_idr = encoder_force_idr.clone();
                let video_track_clone = video_track_clone.clone();
                let webrtc_config = webrtc_config.clone();
                let input_handler = input_handler.clone();
                tokio::spawn(async move {
                    let peer = WebRTCPeer::new(
                        Arc::new(
                            api_clone
                                .new_peer_connection(webrtc_config.clone())
                                .await
                                .unwrap(),
                        ),
                        peer,
                        encoder_force_idr.clone(),
                        input_handler.clone(),
                        video_track_clone.clone(),
                    )
                    .await
                    .expect("Failed to create peer");
                    peers_clone.lock().await.push(peer);
                });
            }
            Result::<()>::Ok(())
        });

        info!("WebRTC initialized");

        Ok(output)
    }
}

#[async_trait]
impl OutputSink for WebRTCOutput {
    async fn write(&mut self, input: Bytes) -> Result<()> {
        self.video_track
            .write_sample(&Sample {
                data: input,
                duration: Duration::from_millis((1000. / self.frame_rate as f64) as u64),
                ..Default::default()
            })
            .await
            .expect("TODO: panic message");
        Ok(())
    }
}
