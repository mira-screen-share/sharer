use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use strum::IntoEnumIterator;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use tokio_util::sync::CancellationToken;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::config::IceServer;
use crate::signaller::{
    AuthenticationPayload, DeclineReason, Signaller, SignallerIceServer, SignallerMessage,
    SignallerMessageDiscriminants, SignallerPeer,
};
use crate::Result;

/// ownership yielded to the user
#[derive(Debug, Clone)]
struct WebSocketSignallerPeer {
    send_queue: Sender<SignallerMessage>,
    topics_rx: Arc<RwLock<HashMap<&'static str, Mutex<broadcast::Receiver<SignallerMessage>>>>>,
    peer_uuid: String,
}

pub struct WebSocketSignaller {
    send_queue: Sender<SignallerMessage>,

    topics_tx: Arc<RwLock<HashMap<&'static str, broadcast::Sender<SignallerMessage>>>>,
    topics_rx: RwLock<HashMap<&'static str, Mutex<broadcast::Receiver<SignallerMessage>>>>,
    room_id: std::sync::Mutex<Option<String>>,

    notify_update: Arc<dyn Fn() + Send + Sync>,
    shutdown_token: CancellationToken,
}

impl WebSocketSignaller {
    async fn process_incoming_message(
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        topics_tx: Arc<RwLock<HashMap<&'static str, broadcast::Sender<SignallerMessage>>>>,
        shutdown_token: CancellationToken,
    ) {
        loop {
            select! {
                msg = read.next() => {
                    if msg.is_none() {
                        break
                    };

                    let msg = msg.unwrap();

                    if let Err(e) = msg {
                        error!("Error reading from websocket: {}", e);
                        break;
                    }

                    trace!("Received websocket message: {:?}", msg);
                    let text = msg.unwrap().into_text().unwrap();
                    match serde_json::from_str::<SignallerMessage>(&text) {
                        Err(e) => {
                            warn!(
                            "Error deserializing websocket message: {}. Message: {}",
                            e, text);
                        }

                        Ok(msg) => {
                            debug!("Deserialized websocket message: {:#?}", msg);
                            if let Some(tx) = topics_tx.read().await.get(msg.clone().into()) {
                                tx.send(msg).unwrap();
                            }
                        }
                    }
                }
                _ = shutdown_token.cancelled() => {
                    break;
                    }
            }
        }
    }
    async fn keepalive(sender: Sender<SignallerMessage>, shutdown_token: CancellationToken) {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            select! {
                _ = ticker.tick() => {}
                _ = shutdown_token.cancelled() => {
                    break;
                }
            }
            if sender.send(SignallerMessage::KeepAlive {}).await.is_err() {
                break;
            }
        }
    }

    async fn process_outgoing(
        mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        mut receiver: Receiver<SignallerMessage>,
        shutdown_token: CancellationToken,
    ) {
        loop {
            select! {
                msg = receiver.recv() => {
                    let text = serde_json::to_string(&msg).unwrap();
                    trace!("Sending websocket message: {:#?}", msg);
                    write.send(Message::text(text)).await.unwrap();
                }
                _ = shutdown_token.cancelled() => {
                    break;
                }
            }
        }
        warn!("Send queue closed");
    }

    async fn gen_rx(
        txs: &RwLock<HashMap<&'static str, broadcast::Sender<SignallerMessage>>>,
    ) -> RwLock<HashMap<&'static str, Mutex<broadcast::Receiver<SignallerMessage>>>> {
        let topics_rx = RwLock::new(HashMap::new());
        for topic in SignallerMessageDiscriminants::iter() {
            let name: &'static str = topic.into();
            let rx = txs.read().await.get(name).unwrap().subscribe();
            topics_rx.write().await.insert(name, Mutex::new(rx));
        }
        topics_rx
    }

    pub async fn new(url: &str, notify_update: Arc<dyn Fn() + Send + Sync>) -> Result<Self> {
        let shutdown_token = CancellationToken::new();
        let (send_queue_sender, send_queue_receiver) = mpsc::channel::<SignallerMessage>(8);

        let topics_tx = Arc::new(RwLock::new(HashMap::new()));
        for topic in SignallerMessageDiscriminants::iter() {
            let (tx, _) = broadcast::channel::<SignallerMessage>(32);
            topics_tx
                .write()
                .await
                .insert(Into::<&'static str>::into(topic), tx);
        }

        let topics_rx = Self::gen_rx(topics_tx.as_ref()).await;

        let url = url::Url::parse(url).unwrap();
        info!("Establishing websocket connection to {}", url);
        let (ws_stream, _) = connect_async(url).await?;

        debug!("Websocket connection established");
        let (write, read) = ws_stream.split();

        // handle all incoming websocket messages
        tokio::spawn(Self::process_incoming_message(
            read,
            topics_tx.clone(),
            shutdown_token.clone(),
        ));

        // handle all outgoing websocket messages
        tokio::spawn(Self::process_outgoing(
            write,
            send_queue_receiver,
            shutdown_token.clone(),
        ));

        // send a keepalive packet every 30 secs
        tokio::spawn(Self::keepalive(
            send_queue_sender.clone(),
            shutdown_token.clone(),
        ));

        Ok(Self {
            send_queue: send_queue_sender,
            topics_tx,
            topics_rx,
            room_id: std::sync::Mutex::new(None),
            notify_update,
            shutdown_token,
        })
    }
}

macro_rules! blocking_recv {
    ($self:ident, $topic:pat, $discriminant:path, $negative:block) => {
        let Ok($topic) = $self
            .topics_rx
            .read()
            .await
            .get($discriminant.into())
            .unwrap()
            .lock()
            .await
            .recv()
            .await
        else {
            $negative
        };
    };
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    async fn start(&self) {
        trace!("Starting session");
        self.send_queue
            .send(SignallerMessage::Start {})
            .await
            .unwrap();
        // waiting for room id
        trace!("Waiting for room id to be assigned");
        blocking_recv!(
            self,
            SignallerMessage::StartResponse { room },
            SignallerMessageDiscriminants::StartResponse,
            {
                return;
            }
        );
        info!("Assigned room id {}", room);
        self.room_id.lock().unwrap().replace(room);
        (self.notify_update)();
    }
    async fn accept_peer_request(&self) -> Option<(String, String, AuthenticationPayload)> {
        blocking_recv!(
            self,
            SignallerMessage::Join { from, name, auth },
            SignallerMessageDiscriminants::Join,
            {
                return None;
            }
        );
        Some((from, name, auth))
    }
    async fn make_new_peer(&self, uuid: String) -> Box<dyn SignallerPeer> {
        Box::new(WebSocketSignallerPeer {
            send_queue: self.send_queue.clone(),
            topics_rx: Arc::new(WebSocketSignaller::gen_rx(self.topics_tx.as_ref()).await),
            peer_uuid: uuid,
        })
    }
    async fn reject_peer_request(&self, viewer_id: String, reason: DeclineReason) {
        self.send_queue
            .send(SignallerMessage::JoinDeclined {
                to: viewer_id,
                reason,
            })
            .await
            .unwrap();
    }
    fn get_room_id(&self) -> Option<String> {
        let room = self.room_id.lock().unwrap();
        room.clone()
    }
    async fn blocking_wait_leave_message(
        &self,
        shutdown_token: CancellationToken,
    ) -> Option<String> {
        let receivers = self.topics_rx.read().await;
        let mut receiver = receivers
            .get(SignallerMessageDiscriminants::Leave.into())
            .unwrap()
            .lock()
            .await;
        select! {
            result = receiver.recv() => {
                if let SignallerMessage::Leave { from } = result.unwrap() {
                    Some(from)
                } else {
                    unreachable!()
                }
            }
            _ = shutdown_token.cancelled() => {
                None
            }
        }
    }
    async fn fetch_ice_servers(&self) -> Vec<SignallerIceServer> {
        self.send_queue
            .send(SignallerMessage::IceServers {})
            .await
            .unwrap();
        trace!("Waiting for ice servers from signaller");
        blocking_recv!(
            self,
            SignallerMessage::IceServersResponse { ice_servers },
            SignallerMessageDiscriminants::IceServersResponse,
            {
                return Vec::new();
            }
        );
        ice_servers
    }
    async fn leave(&self) {
        trace!("Leaving session");
        self.send_queue
            .send(SignallerMessage::Leave {
                from: self.get_room_id().unwrap(),
            })
            .await
            .unwrap();
    }
    async fn kick_viewer(&self, uuid: String) {
        trace!("Kicking viewer {}", uuid);
        self.send_queue
            .send(SignallerMessage::RoomClosed {
                to: uuid,
                room: self.get_room_id().unwrap(),
            })
            .await
            .unwrap();
    }
    async fn close(&self) {
        self.topics_tx.write().await.clear();
        self.topics_rx.write().await.clear();
        self.shutdown_token.cancel();
    }
}

#[async_trait]
impl SignallerPeer for WebSocketSignallerPeer {
    async fn send_offer(&self, offer: &RTCSessionDescription, ice_servers: Vec<IceServer>) {
        trace!("Sending offer");
        self.send_queue
            .send(SignallerMessage::Offer {
                sdp: offer.clone(),
                to: self.peer_uuid.clone(),
                from: "0".to_string(),
                ice_servers,
            })
            .await
            .unwrap();
    }

    async fn recv_answer(&self) -> Option<RTCSessionDescription> {
        timeout(tokio::time::Duration::from_secs(3), async {
            blocking_recv!(
                self,
                SignallerMessage::Answer { sdp, .. },
                SignallerMessageDiscriminants::Answer,
                {
                    return None;
                }
            );
            Some(sdp)
        })
        .await
        .ok()
        .flatten()
    }

    async fn recv_ice_message(&self) -> Option<RTCIceCandidateInit> {
        blocking_recv!(
            self,
            SignallerMessage::Ice { ice, .. },
            SignallerMessageDiscriminants::Ice,
            {
                return None;
            }
        );
        Some(ice)
    }

    async fn send_ice_message(&self, ice: RTCIceCandidateInit) {
        self.send_queue
            .send(SignallerMessage::Ice {
                ice,
                from: "0".to_string(),
                to: self.peer_uuid.clone(),
            })
            .await
            .unwrap();
    }

    fn get_uuid(&self) -> String {
        self.peer_uuid.clone()
    }
}
