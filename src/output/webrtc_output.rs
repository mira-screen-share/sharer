use crate::inputs::InputHandler;
use async_trait::async_trait;
use futures_util::future::join_all;
use log::info;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_credential_type::RTCIceCredentialType::Password;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::output::WebRTCPeer;
use crate::signaller::Signaller;
use crate::OutputSink;
use crate::Result;

pub struct WebRTCOutput {
    api: Arc<webrtc::api::API>,
    peers: Arc<Mutex<Vec<WebRTCPeer>>>,
    send_sample: Sender<Sample>,
    video_track: Arc<TrackLocalStaticSample>,
}

impl WebRTCOutput {
    pub fn make_config() -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: vec![
                RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    ..Default::default()
                },
                RTCIceServer {
                    // TURN server from [Open Relay Project](https://openrelayproject.org)
                    urls: vec!["turn:openrelay.metered.ca:80".to_string()],
                    username: "openrelayproject".to_string(),
                    credential: "openrelayproject".to_string(),
                    credential_type: Password,
                },
            ],
            ..Default::default()
        }
    }

    pub async fn new(
        config: RTCConfiguration,
        mut signaller: Box<dyn Signaller>,
        encoder_force_idr: &mut Arc<AtomicBool>,
        input_handler: Arc<InputHandler>,
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

        let (send_sample, mut recv_sample) = tokio::sync::mpsc::channel::<Sample>(1);

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
            send_sample,
            video_track: video_track.clone(),
        });

        let api_clone = output.api.clone();
        let peers_clone = output.peers.clone();
        let encoder_force_idr = encoder_force_idr.clone();
        let video_track_clone = video_track.clone();
        signaller.start().await;
        tokio::spawn(async move {
            while let Some(peer) = signaller.accept_peer().await {
                peers_clone.lock().await.push(
                    WebRTCPeer::new(
                        Arc::new(api_clone.new_peer_connection(config.clone()).await?),
                        peer,
                        encoder_force_idr.clone(),
                        input_handler.clone(),
                        video_track_clone.clone(),
                    )
                    .await?,
                );
            }
            Result::<()>::Ok(())
        });

        info!("WebRTC initialized");

        Ok(output)
    }
}

#[async_trait]
impl OutputSink for WebRTCOutput {
    async fn write(&mut self, input: &[u8]) -> Result<()> {
        self.video_track
            .write_sample(&Sample {
                data: input.to_vec().into(), // todo: avoid copy
                duration: Duration::from_millis(33),
                ..Default::default()
            })
            .await
            .expect("TODO: panic message");
        Ok(())
    }
}
