use crate::signaller::{AuthenticationPayload, DeclineReason};
use crate::Result;
use anyhow::anyhow;
<<<<<<< Updated upstream
use rand::distributions::Alphanumeric;
=======
use async_trait::async_trait;
use rand::distributions::{Alphanumeric, Distribution};
>>>>>>> Stashed changes
use rand::{thread_rng, Rng};

pub trait Authenticator: Send + Sync {
    /// Return None if authentication is successful
    /// Return Some(reason) if authentication is unsuccessful
    fn authenticate(&self, payload: &AuthenticationPayload) -> Option<DeclineReason>;
}

pub struct PasswordAuthenticator {
    password: String,
}

fn random_user_friendly_string(len: usize) -> String {
    pub struct UserFriendlyAlphabet;
    impl Distribution<u8> for UserFriendlyAlphabet {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
            const RANGE: u32 = 32;
            const GEN_ASCII_STR_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
            GEN_ASCII_STR_CHARSET[(rng.next_u32() >> (32 - 5)) as usize]
        }
    }

    thread_rng()
        .sample_iter(&UserFriendlyAlphabet)
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
        Self::new(random_user_friendly_string(5))
    }

    pub fn password(&self) -> String {
        self.password.clone()
    }
}

impl Authenticator for PasswordAuthenticator {
    fn authenticate(&self, payload: &AuthenticationPayload) -> Option<DeclineReason> {
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
