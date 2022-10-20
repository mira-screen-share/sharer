mod signaller;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::Result;

#[async_trait]
pub trait Signaller {
    /// indicating the start of a session, and starts to accept viewers
    async fn start(&mut self, uuid: String);
    /// get a new peer
    async fn accept_peer(&mut self) -> Result<Box<dyn SignallerPeer>>;
}

#[async_trait]
pub trait SignallerPeer {
    /// send an offer to the peer
    async fn send_offer(&mut self, offer: &RTCSessionDescription);
    /// receive an answer the that peer
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription>;
    /// receive an ice message from the peer
    async fn recv_ice_message(&mut self) -> Option<RTCIceCandidateInit>;
    /// send an ice message to the peer
    async fn send_ice_message(&mut self, ice: RTCIceCandidateInit);
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignallerMessage {
    Offer {
        sdp: RTCSessionDescription,
        uuid: String,
        to: String,
    },
    Answer {
        sdp: RTCSessionDescription,
        uuid: String,
    },
    Join {
        uuid: String,
    },
    Start {
        uuid: String,
    },
    Ice {
        ice: RTCIceCandidateInit,
        uuid: String,
    },
    Leave {
        uuid: String,
    },
}

pub use signaller::WebSocketSignaller;
