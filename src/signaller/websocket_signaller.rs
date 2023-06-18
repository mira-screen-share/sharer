use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::signaller::{Signaller, SignallerMessage, SignallerPeer};
use crate::Result;

/// ownership yielded to the user
#[derive(Debug, Clone)]
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
    peers_receiver: Mutex<Receiver<WebSocketSignallerPeer>>,
    uuid: String,
}

impl WebSocketSignaller {
    async fn process_incoming_message(
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        send_queue_sender: Sender<SignallerMessage>, // send queue for new peers
        peers_sender: Sender<WebSocketSignallerPeer>, // for new peers
        peers: Arc<RwLock<HashMap<String, WebSocketSignallerSender>>>, // existing peers
    ) {
        while let Some(msg) = read.next().await {
            if let Err(e) = msg {
                error!("Error reading from websocket: {}", e);
                break;
            }

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

                    peers.write().await.insert(
                        uuid.clone(),
                        WebSocketSignallerSender {
                            answer_sender,
                            ice_sender,
                        },
                    );
                    debug!("Created new peer {}", uuid);
                    peers_sender
                        .send(WebSocketSignallerPeer {
                            uuid: uuid.clone(),
                            answer_receiver: Arc::new(Mutex::new(answer_receiver)),
                            ice_receiver: Arc::new(Mutex::new(ice_receiver)),
                            send_queue: send_queue_sender.clone(),
                        })
                        .await
                        .unwrap();
                    debug!("Created new peer2 {}", uuid);
                }
                SignallerMessage::Answer { sdp, uuid } => {
                    let sender = {
                        let peer = &peers.read().await[&uuid];
                        let sender = &peer.answer_sender;
                        sender.clone()
                    };
                    sender.send(sdp).await.unwrap();
                }
                SignallerMessage::Ice { ice, uuid, to: _ } => {
                    let sender = {
                        let peer = &peers.read().await[&uuid];
                        let sender = &peer.ice_sender;
                        sender.clone()
                    };
                    sender.send(ice).await.unwrap();
                }
                SignallerMessage::Leave { uuid } => {
                    info!("Peer {} left", uuid);
                }
                _ => {
                    panic!("Unexpected message type");
                }
            };
        }
    }

    async fn keepalive(sender: Sender<SignallerMessage>) {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            ticker.tick().await;
            if sender.send(SignallerMessage::KeepAlive {}).await.is_err() {
                break;
            }
        }
    }

    async fn process_outgoing(
        mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        mut receiver: Receiver<SignallerMessage>,
    ) {
        while let Some(msg) = receiver.recv().await {
            let text = serde_json::to_string(&msg).unwrap();
            trace!("Sending websocket message: {:#?}", msg);
            write.send(Message::text(text)).await.unwrap();
        }
        warn!("Send queue closed");
    }

    pub async fn new(url: &str) -> Result<Self> {
        let (peers_sender, peers_receiver) = mpsc::channel::<WebSocketSignallerPeer>(1);
        let (send_queue_sender, send_queue_receiver) = mpsc::channel::<SignallerMessage>(8);
        let peers = Arc::new(RwLock::new(
            HashMap::<String, WebSocketSignallerSender>::new(),
        ));

        let url = url::Url::parse(url).unwrap();
        info!("Establishing websocket connection to {}", url);
        let (ws_stream, _) = connect_async(url).await?;

        debug!("Websocket connection established");
        let (write, read) = ws_stream.split();

        // create a task to read all incoming websocket messages
        tokio::spawn(Self::process_incoming_message(
            read,
            send_queue_sender.clone(),
            peers_sender,
            peers.clone(),
        ));

        // create a task to handle all outgoing websocket messages
        tokio::spawn(Self::process_outgoing(write, send_queue_receiver));

        // send a keepalive packet every 30 secs
        tokio::spawn(Self::keepalive(send_queue_sender.clone()));

        Ok(Self {
            send_queue: send_queue_sender,
            peers_receiver: Mutex::new(peers_receiver),
            uuid: "my_uuid".into(), // TODO:FIXME
        })
    }
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    async fn start(&self) {
        trace!("Starting session");
        self.send_queue
            .send(SignallerMessage::Start {
                uuid: self.uuid.clone(),
            })
            .await
            .unwrap();
    }
    async fn accept_peer(&self) -> Option<Box<dyn SignallerPeer>> {
        Some(Box::new(self.peers_receiver.lock().await.recv().await?))
    }
}

#[async_trait]
impl SignallerPeer for WebSocketSignallerPeer {
    async fn send_offer(&self, offer: &RTCSessionDescription) {
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
    async fn recv_answer(&self) -> Option<RTCSessionDescription> {
        timeout(
            tokio::time::Duration::from_secs(3),
            self.answer_receiver.lock().await.recv(),
        )
        .await
        .ok()
        .flatten()
    }

    async fn recv_ice_message(&self) -> Option<RTCIceCandidateInit> {
        self.ice_receiver.lock().await.recv().await
    }

    async fn send_ice_message(&self, ice: RTCIceCandidateInit) {
        self.send_queue
            .send(SignallerMessage::Ice {
                ice,
                uuid: "0".to_string(),
                to: self.uuid.clone(),
            })
            .await
            .unwrap();
    }
}
