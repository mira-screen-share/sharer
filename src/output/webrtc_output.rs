use std::sync::Arc;
use std::time::Duration;
use futures_util::SinkExt;

use log::{debug, info};
use tokio::io::BufReader;
use tokio::signal;
use tokio::sync::mpsc::Sender;
use tokio::sync::Notify;
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264};
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::io::h264_reader::H264Reader;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::OutputSink;
use crate::Result;
use crate::signaller::Signaller;

pub struct WebRTCOutput {
    api: webrtc::api::API,
    peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>,
    send_sample: Sender<Sample>,
}

impl WebRTCOutput {
    pub fn make_config(ice_servers: &[String]) -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: ice_servers.iter().map(|url| url.to_owned()).collect(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    pub async fn new(config: RTCConfiguration, signaller: &mut dyn Signaller) -> Result<Self> {
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

        // Create the API object with the MediaEngine
        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        // Create a new RTCPeerConnection
        debug!("Creating peer connection");
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        let notify_tx = Arc::new(Notify::new());
        let notify_video = notify_tx.clone();

        let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);
        let video_done_tx = done_tx.clone();

        debug!("Adding video track");
        // Create a video track
        let video_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                ..Default::default()
            },
            "video".to_owned(),
            "screen".to_owned(),
        ));

        // Add this newly created track to the PeerConnection
        let rtp_sender = peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        // Read incoming RTCP packets
        // Before these packets are returned they are processed by interceptors. For things
        // like NACK this needs to be called.
        tokio::spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
            Result::<()>::Ok(())
        });

        // Set the handler for ICE connection state
        // This will notify you when the peer has connected/disconnected
        peer_connection
            .on_ice_connection_state_change(Box::new(move |connection_state: RTCIceConnectionState| {
                info!("Connection State has changed {}", connection_state);
                if connection_state == RTCIceConnectionState::Connected {
                    notify_tx.notify_waiters();
                }
                Box::pin(async {})
            }))
            .await;

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                info!("Peer Connection State has changed: {}", s);

                if s == RTCPeerConnectionState::Failed {
                    // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                    // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                    // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                    info!("Peer Connection has gone to failed exiting");
                    let _ = done_tx.try_send(());
                }

                Box::pin(async {})
            }))
            .await;


        // Makes an offer, sets the LocalDescription, and starts our UDP listeners
        let offer = peer_connection.create_offer(None).await?;
        debug!("Making an offer: {}", offer.sdp);
        signaller.send_offer(&offer).await;

        info!("Waiting any answers from signaller");
        let answer = signaller.recv_answer().await.unwrap();
        debug!("Received answer: {}", answer.sdp);

        // Set the remote SessionDescription
        peer_connection.set_remote_description(answer).await?;

        // Create channel that is blocked until ICE Gathering is complete
        let mut gather_complete = peer_connection.gathering_complete_promise().await;

        info!("Waiting for ICE gathering to complete");
        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate
        let _ = gather_complete.recv().await;

        let (send_sample, mut recv_sample) = tokio::sync::mpsc::channel::<Sample>(1);

        tokio::spawn(async move {
            while let Some(sample) = recv_sample.recv().await {
                video_track
                    .write_sample(&sample)
                    .await
                    .expect("Failed to write sample");
            }
        });

        info!("WebRTC initialized");
        Ok(Self {
            api,
            peer_connection,
            send_sample,
        })
    }
}

impl OutputSink for WebRTCOutput {
    fn write(&mut self, input: &[u8]) -> Result<()> {
        self.send_sample.send(Sample {
            data: input.to_vec().into(), // todo: avoid copy
            duration: Duration::from_secs(1), // todo: timestamps
            ..Default::default()
        });
        Ok(())
    }
}

impl Drop for WebRTCOutput {
    fn drop(&mut self) {
        self.peer_connection.close();
    }
}
