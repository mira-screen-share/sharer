use crate::inputs::InputHandler;

use log::{debug, info};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;



use webrtc::ice_transport::ice_candidate::RTCIceCandidate;

use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;

use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;


use crate::signaller::SignallerPeer;

use crate::Result;

pub struct WebRTCPeer {
    peer_connection: Arc<RTCPeerConnection>,
    signaller_peer: Box<dyn SignallerPeer>,
}

impl WebRTCPeer {
    pub async fn new(
        peer_connection: Arc<RTCPeerConnection>,
        signaller_peer: Box<dyn SignallerPeer>,
        encoder_force_idr: Arc<AtomicBool>,
        input_handler: Arc<InputHandler>,
        video_track: Arc<TrackLocalStaticSample>,
    ) -> Result<Self> {
        debug!("Initializing a new WebRTC peer");

        peer_connection.add_track(video_track).await?;

        let data_channel = peer_connection.create_data_channel("control", None).await?;
        let input_handler = input_handler.clone();
        data_channel
            .on_message(Box::new(move |msg| {
                let input_handler = input_handler.clone();
                Box::pin(async move {
                    input_handler.sender.send(msg.data).await.unwrap();
                    
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

        info!("WebRTC peer initialized");
        Ok(Self {
            peer_connection,
            signaller_peer,
        })
    }
}
