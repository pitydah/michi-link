//! michi-identity: decentralized identity and trust for the Michi ecosystem.
//!
//! Built on Ed25519 + BLAKE3 (scheme `ed25519-blake3-v1`), with Argon2id KDF
//! and ChaCha20-Poly1305 authenticated at-rest encryption for the private key.
//!
//! - `IdentityManager`: generation, persistence (Argon2id + AEAD + AAD-bound
//!   metadata, atomic writes), automatic migration from legacy formats
//!   (v1 XOR, v2 BLAKE3-KDF), explicit corruption errors.
//! - `DiscoveryEngine`: canonical multicast constants, per-service announce
//!   profiles, deterministic signed announces, timestamp window, replay
//!   protection, UNTRUSTED classification, real fan-out broadcast.
//! - `PairingRegistry`: unified session+PIN flow (challenge, 5-minute expiry,
//!   5 attempts, single use, keyed non-offline-verifiable PIN verifier,
//!   bounded registry with rate limiting).
//! - `QRConnector`: versioned `michi-link-pairing` QR URIs.
//!
//! ## Wire encoding
//!
//! All wire values (michi_id, public_key, signature, nonce) are Base64URL
//! without padding: 43 chars for 32-byte values, 86 chars for signatures.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use michi_identity::init;
//!
//! fn main() {
//!     let config_dir = std::path::Path::new("/home/user/.config/michi");
//!     let (identity, discovery) = init(config_dir, "My Device", "password").unwrap();
//!     println!("Michi ID: {}", identity.michi_id());
//! }
//! ```

pub mod discovery;
pub mod error;
pub mod identity;
pub mod pairing;
pub mod qr;
pub mod types;

use std::sync::Arc;

pub use discovery::DiscoveryEngine;
pub use error::{ContractViolation, IdentityError};
pub use identity::IdentityManager;
pub use pairing::PairingRegistry;
pub use qr::QRConnector;
pub use types::{
    decode_base64url_strict, encode_base64url, Announce, AnnounceProfile, ApiVersion, AuthStrategy,
    IdentityDocument, MichiId, PairConfirmRequest, PairConfirmResponse, PairStartRequest,
    PairStartResponse, PairingQr, PairingSession, Role, Service, TrustLevel,
};

/// Initializes the full identity system.
///
/// 1. Loads (or generates on first run) the Ed25519 keypair.
/// 2. Creates the DiscoveryEngine.
///
/// This is the recommended entry point for Player, Micro Server, etc.
pub fn init(
    config_dir: &std::path::Path,
    device_name: &str,
    password: &str,
) -> Result<(Arc<IdentityManager>, DiscoveryEngine), IdentityError> {
    let identity = Arc::new(IdentityManager::load_or_generate(
        config_dir,
        device_name,
        password,
    )?);
    let discovery = DiscoveryEngine::new(identity.clone());
    Ok((identity, discovery))
}
