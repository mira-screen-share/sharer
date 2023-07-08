use crate::signaller::{AuthenticationPayload, DeclineReason};
use crate::Result;
use anyhow::anyhow;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub trait Authenticator: Send + Sync {
    /// Return None if authentication is successful
    /// Return Some(reason) if authentication is unsuccessful
    fn authenticate(&self, payload: &AuthenticationPayload) -> Option<DeclineReason>;
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

impl Authenticator for PasswordAuthenticator {
    fn authenticate(&self, payload: &AuthenticationPayload) -> Option<DeclineReason> {
        match payload {
            AuthenticationPayload::Password(password) => {
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
