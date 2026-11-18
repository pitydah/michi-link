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

pub mod aircat;
pub mod capabilities;
pub mod discovery;
pub mod error;
pub mod home;
pub mod identity;
pub mod pairing;
pub mod qr;
pub mod types;

use std::sync::Arc;

pub use aircat::{
    canonical_aircat_grant_bytes_v1, validate_aircat_claims_v1, verify_aircat_grant_v1,
    AirCatGrantClaims, AirCatGrantError, AirCatSignedGrantV1,
};
pub use capabilities::{
    is_aircat_original_v1_ready, CanonicalServerCapabilities, AIRCAT_REQUIRED_CAPABILITIES,
    CANONICAL_SERVER_CAPABILITIES, CAP_AIRCAT_AUTHORIZE_V1, CAP_AIRCAT_MANIFEST_V1,
    CAP_AIRCAT_ORIGINAL_DOWNLOAD_V1, CAP_AIRCAT_ORIGINAL_V1, CAP_ARTWORK, CAP_DOWNLOAD, CAP_EVENTS,
    CAP_HOME_TLS_SPKI_PINNED, CAP_LIBRARY, CAP_MULTIROOM, CAP_PLAYBACK, CAP_PLAYLISTS, CAP_QUEUE,
    CAP_RECEIVERS, CAP_SEARCH, CAP_STREAMING, CAP_SYNC, CAP_SYNC_MANIFEST, IDENTITY_SCHEME_V1,
};

pub use discovery::DiscoveryEngine;
pub use error::{ContractViolation, IdentityError};
pub use home::{
    canonical_membership_bytes, canonical_revocation_bytes, device_auth_challenge_payload,
    home_enrollment_pop_payload, server_auth_confirm_payload, sign_device_auth_challenge,
    sign_home_enrollment_pop, sign_server_auth_confirm, verify_device_auth_challenge,
    verify_home_enrollment_pop, verify_membership, verify_revocation, verify_server_auth_confirm,
    verify_server_auth_session, HomeAuthError, HomeRootAuthority, DEVICE_AUTH_DOMAIN_V1,
    HOME_ENROLLMENT_POP_DOMAIN_V1, MEMBERSHIP_DOMAIN_V1, REVOCATION_DOMAIN_V1,
    SERVER_AUTH_DOMAIN_V1,
};
pub use identity::IdentityManager;
pub use pairing::PairingRegistry;
pub use qr::QRConnector;
pub use types::{
    decode_base64url_strict, encode_base64url, Announce, AnnounceProfile, ApiVersion, AuthStrategy,
    DeviceAuthChallengeRequest, DeviceAuthChallengeResponse, DeviceAuthSessionRequest,
    DeviceAuthSessionResponse, DeviceMembershipDto, HomeDeviceRevocationDto, HomeDeviceRosterDto,
    HomeDeviceRosterEntry, HomeEnrollmentApproveRequest, HomeEnrollmentApproveResponse,
    HomeEnrollmentChallengeRequest, HomeEnrollmentChallengeResponse, HomeEnrollmentPendingEntry,
    HomeEnrollmentRequest, HomeEnrollmentRequestResponse, HomeEnrollmentStatus,
    HomeEnrollmentStatusResponse, IdentityDocument, MichiHomeDto, MichiId, PairConfirmRequest,
    PairConfirmResponse, PairStartRequest, PairStartResponse, PairingQr, PairingSession, Role,
    Service, TrustLevel,
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
