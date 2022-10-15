use tokio::sync::mpsc::{Sender, Receiver};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use async_trait::async_trait;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{future, pin_mut, StreamExt};
use serde::{Deserialize, Serialize};
use crate::Result;

#[async_trait]
pub trait Signaller {
    fn send(&self);
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "message_type", content = "payload")]
enum SignallerMessage {
    Answer(RTCSessionDescription),
}

pub struct WebSocketSignaller {
    sdp_receiver: Receiver<RTCSessionDescription>,
}

impl WebSocketSignaller {
    pub async fn new(url: &str) -> Result<Self> {
        let (sdp_sender, sdp_receiver) = tokio::sync::mpsc::channel::<RTCSessionDescription>(1);

        let url = url::Url::parse(&url).unwrap();
        info!("Establishing websocket connection to {}", url);
        let (ws_stream, _) = connect_async(url).await?;
        debug!("Websocket connection established");
        let (write, read) = ws_stream.split();
        tokio::spawn(async move {
            read.for_each(|msg| async {
                trace!("Received websocket message: {:?}", msg);
                let text = msg.unwrap().into_text().unwrap();
                let msg = serde_json::from_str::<SignallerMessage>(&text).unwrap();
                debug!("Deserialized websocket message: {:#?}", msg);
                match msg {
                    SignallerMessage::Answer(answer) => {
                        sdp_sender.send(answer).await.unwrap();
                    }
                }
            }).await;
        });

        Ok(Self {
            sdp_receiver,
        })
    }
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    fn send(&self) {}
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription> {
        self.sdp_receiver.recv().await
    }
}
