mod websocket_signaller;

use async_trait::async_trait;
use dyn_clone::DynClone;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, EnumIter, IntoStaticStr};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[async_trait]
pub trait Signaller: Send + 'static {
    /// indicating the start of a session, and starts to accept viewers
    async fn start(&self);
    /// get a new peer
    async fn accept_peer(&self) -> Option<Box<dyn SignallerPeer>>;
    /// get room id
    fn get_room_id(&self) -> Option<String>;
}

#[async_trait]
pub trait SignallerPeer: DynClone + Send + Sync + 'static {
    /// send an offer to the peer
    async fn send_offer(&self, offer: &RTCSessionDescription);
    /// receive an answer the that peer
    async fn recv_answer(&self) -> Option<RTCSessionDescription>;
    /// receive an ice message from the peer
    async fn recv_ice_message(&self) -> Option<RTCIceCandidateInit>;
    /// send an ice message to the peer
    async fn send_ice_message(&self, ice: RTCIceCandidateInit);
}

#[derive(Debug, Serialize, Deserialize, Clone, IntoStaticStr, EnumIter, EnumDiscriminants)]
#[strum_discriminants(derive(IntoStaticStr, EnumIter))]
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
        to: String,
    },
    Leave {
        uuid: String,
    },
    KeepAlive {},
}

pub use websocket_signaller::WebSocketSignaller;
