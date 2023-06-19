use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::signaller::{Signaller, SignallerMessage, SignallerMessageDiscriminants, SignallerPeer};
use crate::Result;
use strum::IntoEnumIterator;
/// ownership yielded to the user
#[derive(Debug, Clone)]
struct WebSocketSignallerPeer {
    send_queue: Sender<SignallerMessage>,
    topics_rx: Arc<RwLock<HashMap<&'static str, Mutex<broadcast::Receiver<SignallerMessage>>>>>,
    peer_uuid: String,
}

#[derive(Debug)]
pub struct WebSocketSignaller {
    send_queue: Sender<SignallerMessage>,

    topics_tx: Arc<RwLock<HashMap<&'static str, broadcast::Sender<SignallerMessage>>>>,
    topics_rx: RwLock<HashMap<&'static str, Mutex<broadcast::Receiver<SignallerMessage>>>>,
    room_id: std::sync::Mutex<Option<String>>,
}

impl WebSocketSignaller {
    async fn process_incoming_message(
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        topics_tx: Arc<RwLock<HashMap<&'static str, broadcast::Sender<SignallerMessage>>>>,
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
            if let Some(tx) = topics_tx.read().await.get(msg.clone().into()) {
                tx.send(msg).unwrap();
            }
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

    pub async fn new(url: &str) -> Result<Self> {
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
        tokio::spawn(Self::process_incoming_message(read, topics_tx.clone()));

        // handle all outgoing websocket messages
        tokio::spawn(Self::process_outgoing(write, send_queue_receiver));

        // send a keepalive packet every 30 secs
        tokio::spawn(Self::keepalive(send_queue_sender.clone()));

        Ok(Self {
            send_queue: send_queue_sender,
            topics_tx,
            topics_rx,
            room_id: std::sync::Mutex::new(None),
        })
    }
}

macro_rules! blocking_recv {
    ($self:ident, $topic:pat, $discriminant:path) => {
        let $topic = $self.topics_rx
                                                                  .read()
                                                                  .await
                                                                  .get($discriminant.into())
                                                                  .unwrap()
                                                                  .lock()
                                                                  .await
                                                                  .recv()
                                                                  .await
                                                                  .unwrap() else { unreachable!() };
    };
}

#[async_trait]
impl Signaller for WebSocketSignaller {
    fn get_room_id(&self) -> Option<String> {
        let room = self.room_id.lock().unwrap();
        room.clone()
    }
    async fn start(&self) {
        trace!("Starting session");
        let room = uuid::Uuid::new_v4().to_string();
        // TODO: get it from signaller
        self.room_id.lock().unwrap().replace(room.clone());
        self.send_queue
            .send(SignallerMessage::Start { uuid: room })
            .await
            .unwrap();
    }
    async fn accept_peer(&self) -> Option<Box<dyn SignallerPeer>> {
        blocking_recv!(
            self,
            SignallerMessage::Join { uuid },
            SignallerMessageDiscriminants::Join
        );
        Some(Box::new(WebSocketSignallerPeer {
            send_queue: self.send_queue.clone(),
            topics_rx: Arc::new(WebSocketSignaller::gen_rx(self.topics_tx.as_ref()).await),
            peer_uuid: uuid,
        }))
    }
}

#[async_trait]
impl SignallerPeer for WebSocketSignallerPeer {
    async fn send_offer(&self, offer: &RTCSessionDescription) {
        trace!("Sending offer");
        self.send_queue
            .send(SignallerMessage::Offer {
                sdp: offer.clone(),
                to: self.peer_uuid.clone(),
                uuid: "0".to_string(),
            })
            .await
            .unwrap();
    }

    async fn recv_answer(&self) -> Option<RTCSessionDescription> {
        timeout(tokio::time::Duration::from_secs(3), async {
            blocking_recv!(
                self,
                SignallerMessage::Answer { sdp, .. },
                SignallerMessageDiscriminants::Answer
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
            SignallerMessageDiscriminants::Ice
        );
        Some(ice)
    }

    async fn send_ice_message(&self, ice: RTCIceCandidateInit) {
        self.send_queue
            .send(SignallerMessage::Ice {
                ice,
                uuid: "0".to_string(),
                to: self.peer_uuid.clone(),
            })
            .await
            .unwrap();
    }
}
