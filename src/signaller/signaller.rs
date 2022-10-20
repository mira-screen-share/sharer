use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::signaller::{Signaller, SignallerMessage, SignallerPeer};
use crate::Result;

/// ownership yielded to the user
#[derive(Debug)]
struct WebSocketSignallerPeer {
    answer_receiver: Arc<Mutex<Receiver<RTCSessionDescription>>>,
    ice_receiver: Arc<Mutex<Receiver<RTCIceCandidateInit>>>,
    send_queue: Sender<SignallerMessage>,
    uuid: String,
}

/// ownership kept by WebSocketSignaller
#[derive(Debug)]
struct WebSocketSignallerSender {
    answer_sender: Sender<RTCSessionDescription>,
    ice_sender: Sender<RTCIceCandidateInit>,
}

#[derive(Debug)]
pub struct WebSocketSignaller {
    send_queue: Sender<SignallerMessage>,
    peers_receiver: Receiver<WebSocketSignallerPeer>,
    peers: Arc<RwLock<HashMap<String, Mutex<WebSocketSignallerSender>>>>, // uuid -> sender
}

impl WebSocketSignaller {
    pub async fn new(url: &str) -> Result<Self> {
        let (peers_sender, peers_receiver) = mpsc::channel::<WebSocketSignallerPeer>(1);
        let (send_queue_sender, mut send_queue_receiver) = mpsc::channel::<SignallerMessage>(8);
        let mut peers = Arc::new(RwLock::new(HashMap::<
            String,
            Mutex<WebSocketSignallerSender>,
        >::new()));

        let url = url::Url::parse(url).unwrap();
        info!("Establishing websocket connection to {}", url);
        let (ws_stream, _) = connect_async(url).await?;
        debug!("Websocket connection established");
        let (mut write, mut read) = ws_stream.split();

        // create a task to read all incoming websocket messages
        let peers_clone = peers.clone();
        let send_queue_sender_clone = send_queue_sender.clone();
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                trace!("Received websocket message: {:?}", msg);
                let text = msg.unwrap().into_text().unwrap();
                let msg = serde_json::from_str::<SignallerMessage>(&text).unwrap();
                debug!("Deserialized websocket message: {:#?}", msg);
                match msg {
                    SignallerMessage::Join { uuid } => {
                        // create a new peer
                        let (answer_sender, answer_receiver) =
                            mpsc::channel::<RTCSessionDescription>(1);
                        let (ice_sender, ice_receiver) = mpsc::channel::<RTCIceCandidateInit>(4);

                        peers.write().unwrap().deref_mut().insert(
                            uuid.clone(),
                            Mutex::new(WebSocketSignallerSender {
                                answer_sender,
                                ice_sender,
                            }),
                        );
                        peers_sender
                            .send(WebSocketSignallerPeer {
                                uuid,
                                answer_receiver: Arc::new(Mutex::new(answer_receiver)),
                                ice_receiver: Arc::new(Mutex::new(ice_receiver)),
                                send_queue: send_queue_sender_clone.clone(),
                            })
                            .await
                            .unwrap();
                    }
                    SignallerMessage::Answer { sdp, uuid } => {
                        let sender = {
                            let peer = &peers.read().unwrap()[&uuid];
                            let sender = &peer.try_lock().unwrap().answer_sender;
                            sender.clone()
                        };
                        sender.send(sdp).await.unwrap();
                    }
                    SignallerMessage::Ice { ice, uuid } => {
                        if uuid != "0" {
                            let sender = {
                                let peer = &peers.read().unwrap()[&uuid];
                                let sender = &peer.try_lock().unwrap().ice_sender;
                                sender.clone()
                            };
                            sender.send(ice).await.unwrap();
                        }
                    }
                    SignallerMessage::Leave { uuid } => {
                        info!("Peer {} left", uuid);
                    }
                    _ => {
                        panic!("Unexpected message type");
                    }
                };
            }
        });

        // create a task to handle all outgoing websocket messages
        tokio::spawn(async move {
            while let Some(msg) = send_queue_receiver.recv().await {
                let text = serde_json::to_string(&msg).unwrap();
                trace!("Sending websocket message: {:#?}", msg);
                write.send(Message::text(text)).await.unwrap();
            }
            warn!("Send queue closed");
        });

        Ok(Self {
            send_queue: send_queue_sender,
            peers: peers_clone,
            peers_receiver,
        })
    }
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    async fn start(&mut self, uuid: String) {
        trace!("Starting session");
        self.send_queue
            .send(SignallerMessage::Start { uuid })
            .await
            .unwrap();
    }
    async fn accept_peer(&mut self) -> Result<Box<dyn SignallerPeer>> {
        todo!()
    }
}

#[async_trait]
impl SignallerPeer for WebSocketSignallerPeer {
    async fn send_offer(&mut self, offer: &RTCSessionDescription) {
        trace!("Sending offer");
        self.send_queue
            .send(SignallerMessage::Offer {
                sdp: offer.clone(),
                to: self.uuid.clone(),
                uuid: "0".to_string(),
            })
            .await
            .unwrap();
    }
    async fn recv_answer(&mut self) -> Option<RTCSessionDescription> {
        self.answer_receiver.lock().await.recv().await
    }

    async fn recv_ice_message(&mut self) -> Option<RTCIceCandidateInit> {
        todo!()
    }

    async fn send_ice_message(&mut self, ice: RTCIceCandidateInit) {
        self.send_queue
            .send(SignallerMessage::Ice {
                ice,
                uuid: "0".to_string(),
            })
            .await
            .unwrap();
    }
}
