use crate::inputs::InputHandler;
use async_trait::async_trait;
use log::{debug, info};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use webrtc::api::media_engine::MIME_TYPE_H264;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::media::Sample;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::signaller::SignallerPeer;
use crate::OutputSink;
use crate::Result;

pub struct WebRTCPeer {
    peer_connection: Arc<RTCPeerConnection>,
    signaller_peer: Box<dyn SignallerPeer>,
    send_sample: Sender<Sample>,
}

impl WebRTCPeer {
    pub async fn new(
        peer_connection: Arc<RTCPeerConnection>,
        signaller_peer: Box<dyn SignallerPeer>,
        encoder_force_idr: Arc<AtomicBool>,
        input_handler: Arc<InputHandler>,
    ) -> Result<Self> {
        debug!("Initializing a new WebRTC peer");
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
        let _rtp_sender = peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        let data_channel = peer_connection.create_data_channel("control", None).await?;
        let input_handler = input_handler.clone();
        data_channel
            .on_message(Box::new(move |msg| {
                let input_handler = input_handler.clone();
                Box::pin(async move {
                    input_handler.sender.send(msg.data).await.unwrap();
                    ()
                })
            }))
            .await;

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        let encoder_force_idr = encoder_force_idr.clone();
        peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                if s == RTCPeerConnectionState::Connected {
                    // send a keyframe for the newly connected peer so they can
                    // start streaming immediately
                    encoder_force_idr.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Box::pin(async {})
            }))
            .await;

        // Handle ICE messages
        let peer_connection_ice = peer_connection.clone();
        let signaller_peer_ice_read = dyn_clone::clone_box(&*signaller_peer);
        tokio::spawn(async move {
            while let Some(candidate) = signaller_peer_ice_read.recv_ice_message().await {
                trace!("received ICE candidate {:#?}", candidate);
                peer_connection_ice
                    .add_ice_candidate(candidate)
                    .await
                    .unwrap();
            }
        });

        let signaller_peer_ice = dyn_clone::clone_box(&*signaller_peer);
        peer_connection
            .on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
                let signaller_peer_ice = dyn_clone::clone_box(&*signaller_peer_ice);
                Box::pin(async move {
                    if let Some(candidate) = candidate {
                        trace!("ICE candidate {:#?}", candidate);
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

        let (send_sample, mut recv_sample) = tokio::sync::mpsc::channel::<Sample>(1);

        tokio::spawn(async move {
            while let Some(sample) = recv_sample.recv().await {
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

#[async_trait]
impl OutputSink for WebRTCPeer {
    async fn write(&mut self, input: &[u8]) -> Result<()> {
        self.send_sample
            .send(Sample {
                data: input.to_vec().into(),         // todo: avoid copy
                duration: Duration::from_millis(32), // todo: timestamps
                ..Default::default()
            })
            .await?;
        Ok(())
    }
}
