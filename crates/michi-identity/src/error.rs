use thiserror::Error;

/// Canonical error type for the michi-identity crate.
///
/// Error names map to the canonical Michi Link error codes where applicable
/// (`PAIRING_EXPIRED`, `SIGNATURE_INVALID`, `IDENTITY_CORRUPTED`, ...).
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

    /// The identity file cannot be parsed or its format is not supported.
    /// Code: IDENTITY_CORRUPTED.
    #[error("identity file is corrupted: {0}")]
    IdentityCorrupted(String),

    /// AEAD authentication failed. This covers BOTH a wrong password and a
    /// tampered authenticated field: distinguishing them would leak an oracle,
    /// so the crate deliberately returns a single error. Code: IDENTITY_CORRUPTED.
    #[error("identity secret could not be authenticated (wrong password or tampered metadata)")]
    AuthenticationFailed,

    /// A caller-visible wrong-password outcome where the environment can prove
    /// the cause (e.g. a legacy migration attempt with a verified mismatch).
    /// Regular AEAD failures use `AuthenticationFailed` instead.
    #[error("invalid password for identity file")]
    WrongPassword,

    /// Identity metadata was structurally inconsistent before decryption
    /// (invalid KDF parameters, malformed header fields). Code: IDENTITY_CORRUPTED.
    #[error("identity metadata is inconsistent: {0}")]
    MetadataTampered(String),

    /// The decrypted secret key does not match the stored identity fields.
    /// Code: IDENTITY_CORRUPTED.
    #[error("identity keys are inconsistent: {0}")]
    IdentityMismatch(String),

    /// The file uses a format/scheme/KDF version this build cannot handle.
    #[error("unsupported identity format: {0}")]
    UnsupportedFormat(String),

    /// Automatic migration between identity formats failed.
    #[error("identity migration failed: {0}")]
    MigrationFailed(String),

    /// Ed25519 signature verification failed (code SIGNATURE_INVALID).
    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    /// A signed announce replayed a previously seen nonce (code REPLAY_DETECTED).
    #[error("replay detected: nonce already used")]
    ReplayDetected,

    /// Signed announce timestamp outside the allowed freshness window.
    #[error("announce timestamp outside the freshness window")]
    TimestampOutOfWindow,

    /// The announce profile violates the canonical contract rules.
    #[error("contract violation: {0}")]
    ContractViolation(#[from] ContractViolation),

    /// Pairing session expired (code PAIRING_EXPIRED).
    #[error("pairing session has expired")]
    PairingExpired,

    /// Pairing session ran out of attempts (code PAIRING_ATTEMPTS_EXCEEDED).
    #[error("pairing attempts exceeded")]
    PairingAttemptsExceeded,

    /// Pairing session was already consumed and cannot be reused
    /// (code PAIRING_ALREADY_CONSUMED).
    #[error("pairing session already consumed")]
    PairingAlreadyConsumed,

    /// Pairing session does not exist (code PAIRING_NOT_FOUND).
    #[error("pairing session not found")]
    PairingNotFound,

    /// The client identity presented at confirm does not match the one at
    /// start (code PAIRING_KEY_MISMATCH).
    #[error("pairing key mismatch")]
    PairingKeyMismatch,

    /// The PIN does not match (code PAIRING_PIN_MISMATCH).
    #[error("PIN does not match")]
    PairingPinMismatch,

    /// The client challenge signature or nonce failed verification.
    #[error("challenge mismatch")]
    ChallengeMismatch,

    /// A pairing/registry rate limit was exceeded (code RATE_LIMITED).
    #[error("rate limit exceeded")]
    RateLimited,

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

/// Canonical contract violations detected before signing/verifying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ContractViolation {
    /// The service does not support the announced profile shape.
    #[error("invalid service profile for announce")]
    InvalidServiceProfile,
    /// The roles are not allowed for the announced service.
    #[error("invalid role profile for service")]
    InvalidRoleProfile,
    /// The api_version is not allowed for the announced service.
    #[error("invalid api_version for service")]
    InvalidApiVersionProfile,
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
