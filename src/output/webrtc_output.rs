use async_trait::async_trait;
use futures_util::future::join_all;
use futures_util::FutureExt;
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;

use crate::output::WebRTCPeer;
use crate::signaller::{Signaller, SignallerPeer};
use crate::OutputSink;
use crate::Result;

pub struct WebRTCOutput {
    api: webrtc::api::API,
    peers: Arc<Mutex<Vec<WebRTCPeer>>>,
}

impl WebRTCOutput {
    pub fn make_config(ice_servers: &[String]) -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: ice_servers.iter().map(|url| url.to_owned()).collect(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    pub async fn new(config: RTCConfiguration, signaller: &mut impl Signaller) -> Result<Self> {
        info!("Initializing WebRTC");
        // Create a MediaEngine object to configure the supported codec
        let mut m = MediaEngine::default();

        m.register_default_codecs()?;

        // Create a InterceptorRegistry. This is the user configurable RTP/RTCP Pipeline.
        // This provides NACKs, RTCP Reports and other features. If you use `webrtc.NewPeerConnection`
        // this is enabled by default. If you are manually managing You MUST create a InterceptorRegistry
        // for each PeerConnection.
        let mut registry = Registry::new();

        // Use the default set of Interceptors
        registry = register_default_interceptors(registry, &mut m)?;

        // Create the API object with the MediaEngine
        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        let mut peers = Arc::new(Mutex::new(Vec::new()));
        tokio::spawn(async move {
            while let Ok(peer) = signaller.accept_peer().await {
                peers.lock().await.push(
                    WebRTCPeer::new(
                        Arc::new(api.new_peer_connection(config.clone()).await?),
                        peer,
                    )
                    .await?,
                );
            }
            Result::<()>::Ok(())
        });

        info!("WebRTC initialized");

        Ok(Self { api, peers })
    }
}

#[async_trait]
impl OutputSink for WebRTCOutput {
    async fn write(&mut self, input: &[u8]) -> Result<()> {
        let mut peers = self.peers.lock_owned().await;
        join_all(peers.iter_mut().map(|peer| peer.write(input))).await;
        Ok(())
    }
}
