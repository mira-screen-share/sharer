use tokio::sync::mpsc::{Sender, Receiver};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use async_trait::async_trait;
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite::protocol::Message, WebSocketStream};
use futures_util::{future, pin_mut, Sink, SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Error as WsError;
use crate::Result;

#[async_trait]
pub trait Signaller {
    async fn send_offer(&mut self, offer: &RTCSessionDescription);
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "message_type", content = "payload")]
enum SignallerMessage {
    Offer(RTCSessionDescription),
    Answer(RTCSessionDescription),
}

pub struct WebSocketSignaller {
    answer_receiver: Receiver<RTCSessionDescription>,
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
}

impl WebSocketSignaller {
    pub async fn new(url: &str) -> Result<Self> {
        let (sdp_answer_sender, sdp_answer_receiver) = tokio::sync::mpsc::channel::<RTCSessionDescription>(1);

        let url = url::Url::parse(&url).unwrap();
        info!("Establishing websocket connection to {}", url);
        let (ws_stream, _) = connect_async(url).await?;
        debug!("Websocket connection established");
        let (mut write, read) = ws_stream.split();
        tokio::spawn(async move {
            read.for_each(|msg| async {
                trace!("Received websocket message: {:?}", msg);
                let text = msg.unwrap().into_text().unwrap();
                let msg = serde_json::from_str::<SignallerMessage>(&text).unwrap();
                debug!("Deserialized websocket message: {:#?}", msg);
                match msg {
                    SignallerMessage::Answer(answer) => {
                        sdp_answer_sender.send(answer).await.unwrap();
                    }
                    _ => {
                        panic!("Unexpected message type");
                    }
                }
            }).await;
        });

        Ok(Self {
            answer_receiver: sdp_answer_receiver,
            write,
        })
    }
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    async fn send_offer(&mut self, offer: &RTCSessionDescription) {
        trace!("Sending offer");
        self.write.send(Message::Text(serde_json::to_string(&SignallerMessage::Offer(offer.clone())).unwrap())).await.unwrap();
    }
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription> {
        self.answer_receiver.recv().await
    }
}
