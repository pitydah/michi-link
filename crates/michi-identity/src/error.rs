use thiserror::Error;

/// Canonical error type for the michi-identity crate.
///
/// Error names map to the canonical Michi Link error codes where
/// applicable (`PAIRING_EXPIRED`, `SIGNATURE_INVALID`, ...).
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

    /// The identity file exists but cannot be decrypted or parsed.
    /// Never silently regenerates the identity (code IDENTITY_CORRUPTED).
    #[error("identity file is corrupted: {0}")]
    IdentityCorrupted(String),

    /// Wrong password provided for the identity file.
    #[error("invalid password for identity file")]
    InvalidPassword,

    /// Authenticated decryption failed (tampered ciphertext, nonce or AAD).
    #[error("identity secret could not be authenticated: {0}")]
    AeadFailure(String),

    /// Automatic migration from the legacy (v1) identity format failed.
    #[error("identity migration from legacy format failed: {0}")]
    MigrationFailed(String),

    /// The stored michi_id does not match the derived identity.
    #[error("identity keys are inconsistent: {0}")]
    KeyMismatch(String),

    /// Ed25519 signature verification failed (code SIGNATURE_INVALID).
    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    /// A signed announce replayed a previously seen nonce (code REPLAY_DETECTED).
    #[error("replay detected: nonce already used")]
    ReplayDetected,

    /// Signed announce timestamp outside the allowed freshness window.
    #[error("announce timestamp outside the freshness window")]
    TimestampOutOfWindow,

    /// Pairing session expired (code PAIRING_EXPIRED).
    #[error("pairing session has expired")]
    PairingExpired,

    /// Pairing session ran out of attempts (code PAIRING_ATTEMPTS_EXCEEDED).
    #[error("pairing attempts exceeded")]
    PairingAttemptsExceeded,

    /// Pairing session was already consumed and cannot be reused.
    #[error("pairing session already consumed")]
    PairingAlreadyConsumed,

    /// Pairing session does not exist.
    #[error("pairing session not found")]
    PairingNotFound,

    /// The client identity presented at confirm does not match the one at start.
    #[error("pairing key mismatch")]
    PairingKeyMismatch,

    /// The PIN does not match.
    #[error("PIN does not match")]
    PinMismatch,

    #[error("challenge nonce mismatch")]
    ChallengeMismatch,

    #[error("peer public key not found")]
    PeerNotFound,

    #[error("failed to parse pairing URI: {0}")]
    UriParseFailed(String),

    /// QR URI exceeds the maximum allowed size.
    #[error("pairing QR URI exceeds the maximum allowed size")]
    QrTooLarge,

    /// The pairing QR endpoint uses a disallowed URL scheme.
    #[error("pairing endpoint scheme not allowed")]
    UrlSchemeNotAllowed,

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
