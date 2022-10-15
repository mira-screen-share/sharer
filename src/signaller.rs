use tokio::sync::mpsc::{Sender, Receiver};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use async_trait::async_trait;

#[async_trait]
pub trait Signaller {
    fn send(&self);
    async fn recv_offer(&mut self) -> Option<RTCSessionDescription>;
}

pub struct WebSocketSignaller {
    sdp_sender: Sender<RTCSessionDescription>,
    sdp_receiver: Receiver<RTCSessionDescription>,
}

impl WebSocketSignaller {
    pub fn new() -> Self {
        //serde_json::from_str::<RTCSessionDescription>(&desc_data)?
        let (sdp_sender, sdp_receiver) = tokio::sync::mpsc::channel::<RTCSessionDescription>(1);
        Self {
            sdp_sender,
            sdp_receiver,
        }
    }
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    fn send(&self) {}
    async fn recv_offer(&mut self) -> Option<RTCSessionDescription> {
        self.sdp_receiver.recv().await
    }
}
