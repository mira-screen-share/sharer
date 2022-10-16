use tokio::sync::mpsc::{Sender, Receiver};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use async_trait::async_trait;
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite::protocol::Message, WebSocketStream};
use futures_util::{future, pin_mut, Sink, SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Error as WsError;
use webrtc::ice::candidate::Candidate;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use crate::Result;

#[async_trait]
pub trait Signaller {
    async fn send_offer(&mut self, offer: &RTCSessionDescription, to: String);
    async fn start(&mut self, uuid: String);
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription>;
    async fn recv_join(&mut self) -> Option<String>;
    fn recv_ice_channel(&mut self) -> Receiver<RTCIceCandidateInit>;
    fn sender(&mut self) -> Sender<SignallerMessage>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignallerMessage {
    Offer { sdp: RTCSessionDescription, uuid: String, to: String },
    Answer { sdp: RTCSessionDescription },
    Join { uuid: String },
    Start { uuid: String },
    Ice { ice: RTCIceCandidateInit, uuid: String },
    Leave { uuid: String },
}

pub struct WebSocketSignaller {
    answer_receiver: Receiver<RTCSessionDescription>,
    join_receiver: Receiver<String>,
    ice_receiver: Option<Receiver<RTCIceCandidateInit>>,
   // write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    send_queue: Sender<SignallerMessage>,
}

impl WebSocketSignaller {
    pub async fn new(url: &str) -> Result<Self> {
        let (sdp_answer_sender, sdp_answer_receiver) = tokio::sync::mpsc::channel::<RTCSessionDescription>(1);
        let (join_sender, join_receiver) = tokio::sync::mpsc::channel::<String>(1);
        let (ice_sender, ice_receiver) = tokio::sync::mpsc::channel::<RTCIceCandidateInit>(8);
        let (send_queue_sender, mut send_queue_receiver) = tokio::sync::mpsc::channel::<SignallerMessage>(8);

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
                    SignallerMessage::Join { uuid } => {
                        join_sender.send(uuid).await.unwrap();
                    }
                    SignallerMessage::Answer { sdp } => {
                        sdp_answer_sender.send(sdp).await.unwrap();
                    }
                    SignallerMessage::Ice { ice, uuid } => {
                        if uuid != "0" {
                            ice_sender.send(ice).await.unwrap();
                        }
                    }
                    SignallerMessage::Leave { uuid } => {
                        info!("Peer {} left", uuid);
                    }
                    _ => {
                        panic!("Unexpected message type");
                    }
                }
            }).await;
        });

        tokio::spawn(async move {
            while let Some(msg) = send_queue_receiver.recv().await {
                let text = serde_json::to_string(&msg).unwrap();
                trace!("Sending websocket message: {:#?}", msg);
                write.send(Message::text(text)).await.unwrap();
            };
            warn!("Send queue closed");
        });

        Ok(Self {
            answer_receiver: sdp_answer_receiver,
            join_receiver,
            send_queue: send_queue_sender,
            ice_receiver: Some(ice_receiver),
        })
    }
    pub(crate) async fn send_ice(ice: &RTCIceCandidate, sender: &mut Sender<SignallerMessage>) {
        trace!("Sending ice {:#?}", ice);
        //self.write.send(Message::Text(serde_json::to_string(
        //    &SignallerMessage::Ice { ice: ice.to_json().await.unwrap(), uuid: "0".to_string() }).unwrap()
        //)).await.unwrap();
        sender.send(SignallerMessage::Ice { ice: ice.to_json().await.unwrap(), uuid: "0".to_string() }).await.unwrap();
    }
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    async fn send_offer(&mut self, offer: &RTCSessionDescription, to: String) {
        trace!("Sending offer");
        //self.write.send(Message::Text(serde_json::to_string(
        //    &SignallerMessage::Offer { sdp: offer.clone(), to }).unwrap()
        //)).await.unwrap();
        self.send_queue.send(SignallerMessage::Offer { sdp: offer.clone(), to, uuid: "0".to_string() }).await.unwrap();
    }
    async fn start(&mut self, uuid: String) {
        trace!("Starting session");
        //self.write.send(Message::Text(serde_json::to_string(
        //    &SignallerMessage::Start { uuid }).unwrap()
        //)).await.unwrap();
        self.send_queue.send(SignallerMessage::Start { uuid }).await.unwrap();
    }
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription> {
        self.answer_receiver.recv().await
    }
    async fn recv_join(&mut self) -> Option<String> {
        self.join_receiver.recv().await
    }
    fn recv_ice_channel(&mut self) -> Receiver<RTCIceCandidateInit> {
        return std::mem::replace(&mut self.ice_receiver, None).unwrap();
    }
    fn sender(&mut self) -> Sender<SignallerMessage> {
        self.send_queue.clone()
    }
}
