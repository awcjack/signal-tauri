//! Signal protocol integration using presage

pub mod manager;
pub mod messages;
pub mod contacts;
pub mod groups;
pub mod attachments;
pub mod provisioning;
pub mod registration;
pub mod backup;

pub use manager::{ConnectionState, SignalEvent, SignalManager};

use thiserror::Error;

/// Signal-related errors
#[derive(Error, Debug)]
pub enum SignalError {
    #[error("Not registered")]
    NotRegistered,

    #[error("Already registered")]
    AlreadyRegistered,

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Linking failed: {0}")]
    LinkingFailed(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Message send failed: {0}")]
    SendFailed(String),

    #[error("Message receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Attachment error: {0}")]
    AttachmentError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for SignalError {
    fn from(err: anyhow::Error) -> Self {
        SignalError::Unknown(err.to_string())
    }
}
