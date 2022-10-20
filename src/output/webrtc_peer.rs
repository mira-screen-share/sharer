use futures_util::SinkExt;
use std::sync::Arc;
use std::time::Duration;

use log::{debug, info};

use tokio::sync::mpsc::Sender;
use tokio::sync::Notify;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;

use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::signaller::{Signaller, SignallerPeer, WebSocketSignaller};
use crate::Result;
use crate::{OutputSink, WebRTCOutput};

pub struct WebRTCPeer {
    peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>,
    signaller_peer: Box<dyn SignallerPeer>,
    send_sample: Sender<Sample>,
}

impl WebRTCPeer {
    pub async fn new<T: SignallerPeer + Send + Sync + Clone + 'static>(
        peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>,
        mut signaller_peer: Box<T>,
    ) -> Result<Self> {
        debug!("Initializing a new WebRTC peer");
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
            .on_ice_connection_state_change(Box::new(
                move |connection_state: RTCIceConnectionState| {
                    info!("Connection State has changed {}", connection_state);
                    Box::pin(async {})
                },
            ))
            .await;

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                info!("Peer Connection State has changed: {}", s);
                Box::pin(async {})
            }))
            .await;

        // Handle ICE messages
        let mut peer_connection_ice = peer_connection.clone();
        let mut signaller_peer_ice_read = signaller_peer.clone();
        tokio::spawn(async move {
            while let candidate = signaller_peer_ice_read.recv_ice_message().await {
                debug!("received ICE candidate {:#?}", candidate);
                if let Some(candidate) = candidate {
                    peer_connection_ice
                        .add_ice_candidate(candidate)
                        .await
                        .unwrap();
                } else {
                    break;
                }
            }
        });

        let mut signaller_peer_ice = signaller_peer.clone();
        peer_connection
            .on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
                let mut signaller_peer_ice = signaller_peer_ice.clone();
                Box::pin(async move {
                    if let Some(candidate) = candidate {
                        debug!("ICE candidate {:#?}", candidate);
                        signaller_peer_ice
                            .send_ice_message(candidate.to_json().await.unwrap())
                            .await;
                    }
                })
            }))
            .await;

        // Makes an offer, sets the LocalDescription, and starts our UDP listeners
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;
        trace!("Making an offer: {}", offer.sdp);
        signaller_peer.send_offer(&offer).await;

        info!("Waiting any answers from signaller");
        let answer = signaller_peer.recv_answer().await.unwrap();
        trace!("Received answer: {}", answer.sdp);

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
                //debug!("Sending sample");
                video_track
                    .write_sample(&sample)
                    .await
                    .expect("Failed to write sample");
            }
            warn!("Video track closed");
        });

        info!("WebRTC peer initialized");
        Ok(Self {
            peer_connection,
            signaller_peer,
            send_sample,
        })
    }
}

impl OutputSink for WebRTCPeer {
    fn write(&mut self, input: &[u8]) -> Result<()> {
        self.send_sample
            .try_send(Sample {
                data: input.to_vec().into(),         // todo: avoid copy
                duration: Duration::from_millis(32), // todo: timestamps
                ..Default::default()
            })
            .expect("TODO: panic message");
        Ok(())
    }
}

impl Drop for WebRTCPeer {
    fn drop(&mut self) {
        self.peer_connection.close();
    }
}
