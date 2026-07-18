use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("failed to generate Ed25519 keypair: {0}")]
    KeyGenerationFailed(String),

    #[error("identity key file not found: {0}")]
    KeyNotFound(String),

    #[error("failed to load identity key: {0}")]
    KeyLoadFailed(String),

    #[error("failed to save identity key: {0}")]
    KeySaveFailed(String),

    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    #[error("PIN does not match")]
    PinMismatch,

    #[error("pairing PIN has expired")]
    PinExpired,

    #[error("challenge nonce mismatch")]
    ChallengeMismatch,

    #[error("peer public key not found")]
    PeerNotFound,

    #[error("failed to parse pairing URI: {0}")]
    UriParseFailed(String),

    #[error("invalid nonce: {0}")]
    InvalidNonce(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<ed25519_dalek::SignatureError> for IdentityError {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        IdentityError::InvalidSignature(e.to_string())
    }
}

impl From<std::io::Error> for IdentityError {
    fn from(e: std::io::Error) -> Self {
        IdentityError::Internal(e.to_string())
    }
}

impl From<rmp_serde::encode::Error> for IdentityError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        IdentityError::Internal(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for IdentityError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        IdentityError::Internal(e.to_string())
    }
}

impl From<base64::DecodeError> for IdentityError {
    fn from(e: base64::DecodeError) -> Self {
        IdentityError::Internal(e.to_string())
    }
}
