use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::output::WebRTCOutput;
use crate::signaller::{AuthenticationPayload, DeclineReason};
use crate::Result;

#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Return None if authentication is successful
    /// Return Some(reason) if authentication is unsuccessful
    async fn authenticate(
        &self,
        uuid: String,
        payload: &AuthenticationPayload,
    ) -> Option<DeclineReason>;
}

#[derive(Clone, Debug)]
pub struct ViewerIdentifier {
    pub uuid: String,
    pub name: String,
}

pub struct PasswordAuthenticator {
    password: String,
}

fn random_string(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

impl PasswordAuthenticator {
    pub fn new(password: String) -> Result<Self> {
        if password.is_empty() {
            return Err(anyhow!("Password cannot be empty"));
        }
        Ok(Self { password })
    }

    pub fn random() -> Result<Self> {
        Self::new(random_string(8))
    }

    pub fn password(&self) -> String {
        self.password.clone()
    }
}

#[async_trait]
impl Authenticator for PasswordAuthenticator {
    async fn authenticate(
        &self,
        _uuid: String,
        payload: &AuthenticationPayload,
    ) -> Option<DeclineReason> {
        match payload {
            AuthenticationPayload::Password { password } => {
                if *password == self.password {
                    None
                } else {
                    Some(DeclineReason::IncorrectPassword)
                }
            }
            _ => Some(DeclineReason::NoCredentials),
        }
    }
}

pub struct ViewerManager {
    viewing_viewers: Mutex<Vec<ViewerIdentifier>>,
    pending_viewers: Mutex<Vec<ViewerIdentifier>>,
    auth_result_senders: Mutex<HashMap<String, Sender<bool>>>,
    notify_update: Arc<dyn Fn() + Send + Sync>,
    webrtc_output: Mutex<Option<Arc<Mutex<WebRTCOutput>>>>,
}

impl ViewerManager {
    pub fn new(notify_update: Arc<dyn Fn() + Send + Sync>) -> ViewerManager {
        ViewerManager {
            viewing_viewers: Mutex::new(Vec::new()),
            pending_viewers: Mutex::new(Vec::new()),
            auth_result_senders: Mutex::new(HashMap::new()),
            notify_update,
            webrtc_output: Mutex::new(None),
        }
    }
    pub async fn get_viewing_viewers(&self) -> Vec<ViewerIdentifier> {
        self.viewing_viewers.lock().await.clone()
    }
    pub async fn get_pending_viewers(&self) -> Vec<ViewerIdentifier> {
        self.pending_viewers.lock().await.clone()
    }
    async fn send_viewer_auth_result(&self, viewer: ViewerIdentifier, permit: bool) {
        self.auth_result_senders
            .lock()
            .await
            .get(&viewer.uuid)
            .unwrap()
            .send(permit)
            .await
            .expect("failed to send result");
    }
    pub async fn permit_viewer(&self, viewer: ViewerIdentifier) {
        self.send_viewer_auth_result(viewer, true).await;
        (self.notify_update)();
    }
    pub async fn decline_viewer(&self, viewer: ViewerIdentifier) {
        self.send_viewer_auth_result(viewer, false).await;
        (self.notify_update)();
    }
    pub async fn clear(&self) {
        self.viewing_viewers.lock().await.clear();
        self.pending_viewers.lock().await.clear();
        self.auth_result_senders.lock().await.clear();
        self.webrtc_output.lock().await.take();
    }
    pub async fn kick_viewer(&self, viewer: ViewerIdentifier) {
        let output = self.webrtc_output.lock().await;
        if let Some(output) = output.as_ref() {
            output.lock().await.kick_peer(&viewer.uuid).await;
            self.viewer_left(&viewer.uuid).await;
        }
    }

    pub async fn viewer_left(&self, viewer_uuid: &String) {
        self.viewing_viewers
            .lock()
            .await
            .retain(|v| v.uuid != *viewer_uuid);
        self.pending_viewers
            .lock()
            .await
            .retain(|v| v.uuid != *viewer_uuid);
        (self.notify_update)();
    }

    pub async fn set_webrtc_output(&self, output: Arc<Mutex<WebRTCOutput>>) {
        *self.webrtc_output.lock().await = Some(output);
    }
}

#[async_trait]
impl Authenticator for ViewerManager {
    async fn authenticate(
        &self,
        uuid: String,
        _payload: &AuthenticationPayload,
    ) -> Option<DeclineReason> {
        let viewer = ViewerIdentifier {
            uuid: uuid.clone(),
            name: uuid.clone(),
        }; // todo: get name
        self.pending_viewers.lock().await.push(viewer.clone());
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        self.auth_result_senders
            .lock()
            .await
            .insert(uuid.clone(), sender.clone());
        // wait for a decision
        info!("{} is waiting for authentication", uuid);
        (self.notify_update)();
        let decision = receiver.recv().await.unwrap();
        self.auth_result_senders.lock().await.remove(&uuid);
        self.pending_viewers.lock().await.retain(|v| v.uuid != uuid);
        info!("{} got authentication decision: {}", uuid, decision);
        if decision {
            self.viewing_viewers.lock().await.push(viewer);
            None
        } else {
            Some(DeclineReason::UserDeclined)
        }
    }
}

pub struct ComplexAuthenticator {
    authenticators: Vec<Arc<dyn Authenticator>>,
}

impl ComplexAuthenticator {
    pub fn new(authenticators: Vec<Arc<dyn Authenticator>>) -> Self {
        Self { authenticators }
    }
}

#[async_trait]
impl Authenticator for ComplexAuthenticator {
    async fn authenticate(
        &self,
        uuid: String,
        payload: &AuthenticationPayload,
    ) -> Option<DeclineReason> {
        for authenticator in &self.authenticators {
            if let Some(reason) = authenticator.authenticate(uuid.clone(), payload).await {
                return Some(reason);
            }
        }
        None
    }
}
