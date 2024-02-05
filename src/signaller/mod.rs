mod websocket_signaller;

use async_trait::async_trait;
use dyn_clone::DynClone;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, EnumIter, IntoStaticStr};
use tokio_util::sync::CancellationToken;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct SignallerIceServer {
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[async_trait]
pub trait Signaller: Send + 'static {
    /// indicating the start of a session, and starts to accept viewers
    async fn start(&self);
    /// get a new peer request
    async fn accept_peer_request(&self) -> Option<(String, String, AuthenticationPayload)>;
    /// make a new peer
    async fn make_new_peer(&self, uuid: String) -> Box<dyn SignallerPeer>;
    /// reject peer connection request
    async fn reject_peer_request(&self, viewer_id: String, reason: DeclineReason);
    /// get room id
    fn get_room_id(&self) -> Option<String>;
    /// get leave message. returns uuid of the viewer left.
    async fn blocking_wait_leave_message(
        &self,
        shutdown_token: CancellationToken,
    ) -> Option<String>;
    /// fetch ice servers
    async fn fetch_ice_servers(&self) -> Vec<SignallerIceServer>;
    /// indicating the leaving of a session
    async fn leave(&self);
    /// Remove a viewer from the session
    async fn kick_viewer(&self, uuid: String);
    /// close signaller
    async fn close(&self);
}

#[async_trait]
pub trait SignallerPeer: DynClone + Send + Sync + 'static {
    /// send an offer to the peer
    async fn send_offer(&self, offer: &RTCSessionDescription, ice_servers: Vec<IceServer>);
    /// receive an answer the that peer
    async fn recv_answer(&self) -> Option<RTCSessionDescription>;
    /// receive an ice message from the peer
    async fn recv_ice_message(&self) -> Option<RTCIceCandidateInit>;
    /// send an ice message to the peer
    async fn send_ice_message(&self, ice: RTCIceCandidateInit);
    /// get uuid
    fn get_uuid(&self) -> String;
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthenticationPayload {
    #[default]
    None,
    Password {
        password: String,
    },
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum DeclineReason {
    #[default]
    Unknown = 0,
    IncorrectPassword = 1,
    NoCredentials = 2,
    UserDeclined = 3,
}

#[derive(Debug, Serialize, Deserialize, Clone, IntoStaticStr, EnumIter, EnumDiscriminants)]
#[strum_discriminants(derive(IntoStaticStr, EnumIter))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignallerMessage {
    Offer {
        sdp: RTCSessionDescription,
        from: String,
        to: String,
        ice_servers: Vec<IceServer>,
    },
    Answer {
        sdp: RTCSessionDescription,
        from: String,
    },
    Join {
        from: String, // viewer uuid
        name: String, // viewer name
        auth: AuthenticationPayload,
    },
    Start {},
    StartResponse {
        room: String,
    },
    Ice {
        ice: RTCIceCandidateInit,
        from: String,
        to: String,
    },
    RoomClosed {
        to: String,
        room: String,
    },
    Leave {
        from: String,
    },
    JoinDeclined {
        reason: DeclineReason,
        to: String,
    },
    KeepAlive {},
    IceServers {},
    IceServersResponse {
        ice_servers: Vec<SignallerIceServer>,
    },
}

use crate::config::IceServer;
pub use websocket_signaller::WebSocketSignaller;
