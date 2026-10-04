//! Michi Home and Device Authentication (Trust Architecture V2).
//!
//! Provides cryptographic primitives for:
//! - Home Root Authority (root key generation, membership issuance, device revocation)
//! - Device Membership verification (signature under root authority, key-identity derivation coherence)
//! - Domain-separated mutual device authentication challenge-response:
//!   `"michi-link-device-auth-v1" || home_id || server_michi_id || client_michi_id || challenge_id || challenge_nonce`
//! - Server confirmation mutual authentication:
//!   `"michi-link-server-auth-v1" || home_id || server_michi_id || client_michi_id || challenge_id || session_token`

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use thiserror::Error;

use crate::identity::IdentityManager;
use crate::types::{
    decode_base64url_strict, encode_base64url, DeviceMembershipDto, HomeDeviceRevocationDto,
    MichiId, Role,
};

/// Domain separation prefix for device authentication signatures.
pub const DEVICE_AUTH_DOMAIN_V1: &[u8] = b"michi-link-device-auth-v1";
/// Domain separation prefix for server mutual session confirmations.
pub const SERVER_AUTH_DOMAIN_V1: &[u8] = b"michi-link-server-auth-v1";
/// Domain separation prefix for membership certificates.
pub const MEMBERSHIP_DOMAIN_V1: &[u8] = b"michi-link-membership-v1";
/// Domain separation prefix for device revocations.
pub const REVOCATION_DOMAIN_V1: &[u8] = b"michi-link-revocation-v1";

#[derive(Debug, Error)]
pub enum HomeAuthError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Public key decode error")]
    InvalidPublicKey,
    #[error("Signature decode error")]
    InvalidSignatureEncoding,
    #[error("Device michi_id mismatch: does not derive from announced public key")]
    IdentityMismatch,
    #[error("Home ID mismatch: expected {expected}, got {got}")]
    HomeIdMismatch { expected: String, got: String },
    #[error("Device is revoked in this home")]
    DeviceRevoked,
    #[error("Unsupported membership version: {0}")]
    UnsupportedVersion(u32),
}

/// Home Root Authority keypair managing device memberships and revocations.
pub struct HomeRootAuthority {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl HomeRootAuthority {
    /// Generates a fresh random Home Root Authority keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Constructs an authority from an existing 32-byte Ed25519 seed or signing key.
    pub fn from_signing_key(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Public key in base64url (43 chars, no padding).
    pub fn public_key_base64url(&self) -> String {
        encode_base64url(&self.verifying_key.to_bytes())
    }

    /// Returns the home_id derived from the root public key: blake3(pk).
    pub fn home_id(&self) -> String {
        MichiId::from_public_key(&self.verifying_key).to_base64url()
    }

    /// Issues a signed membership certificate for a device in this home.
    #[allow(clippy::too_many_arguments)]
    pub fn issue_membership(
        &self,
        home_id: &str,
        device_michi_id: &str,
        device_public_key: &str,
        device_type: &str,
        roles: Vec<Role>,
        issued_at: &str,
        serial: u64,
    ) -> DeviceMembershipDto {
        let canonical_bytes = canonical_membership_bytes(
            home_id,
            device_michi_id,
            device_public_key,
            device_type,
            &roles,
            issued_at,
            serial,
        );
        let signature = encode_base64url(&self.signing_key.sign(&canonical_bytes).to_bytes());
        DeviceMembershipDto {
            version: 1,
            home_id: home_id.to_string(),
            device_michi_id: device_michi_id.to_string(),
            device_public_key: device_public_key.to_string(),
            device_type: device_type.to_string(),
            roles,
            issued_at: issued_at.to_string(),
            serial,
            signature,
        }
    }

    /// Issues a signed device revocation record for this home.
    pub fn issue_revocation(
        &self,
        home_id: &str,
        revoked_device_michi_id: &str,
        revoked_at: &str,
        reason: &str,
    ) -> HomeDeviceRevocationDto {
        let canonical_bytes =
            canonical_revocation_bytes(home_id, revoked_device_michi_id, revoked_at, reason);
        let signature = encode_base64url(&self.signing_key.sign(&canonical_bytes).to_bytes());
        HomeDeviceRevocationDto {
            version: 1,
            home_id: home_id.to_string(),
            revoked_device_michi_id: revoked_device_michi_id.to_string(),
            revoked_at: revoked_at.to_string(),
            reason: reason.to_string(),
            signature,
        }
    }
}

/// Constructs canonical deterministic bytes for signing a membership certificate.
#[allow(clippy::too_many_arguments)]
pub fn canonical_membership_bytes(
    home_id: &str,
    device_michi_id: &str,
    device_public_key: &str,
    device_type: &str,
    roles: &[Role],
    issued_at: &str,
    serial: u64,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(MEMBERSHIP_DOMAIN_V1);
    buf.extend_from_slice(home_id.as_bytes());
    buf.extend_from_slice(device_michi_id.as_bytes());
    buf.extend_from_slice(device_public_key.as_bytes());
    buf.extend_from_slice(device_type.as_bytes());
    let mut role_names: Vec<&'static str> = roles.iter().map(|r| r.as_str()).collect();
    role_names.sort_unstable();
    for role in role_names {
        buf.extend_from_slice(b":");
        buf.extend_from_slice(role.as_bytes());
    }
    buf.extend_from_slice(b":");
    buf.extend_from_slice(issued_at.as_bytes());
    buf.extend_from_slice(b":");
    buf.extend_from_slice(serial.to_string().as_bytes());
    buf
}

/// Constructs canonical deterministic bytes for signing a revocation record.
pub fn canonical_revocation_bytes(
    home_id: &str,
    revoked_device_michi_id: &str,
    revoked_at: &str,
    reason: &str,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(REVOCATION_DOMAIN_V1);
    buf.extend_from_slice(home_id.as_bytes());
    buf.extend_from_slice(revoked_device_michi_id.as_bytes());
    buf.extend_from_slice(b":");
    buf.extend_from_slice(revoked_at.as_bytes());
    buf.extend_from_slice(b":");
    buf.extend_from_slice(reason.as_bytes());
    buf
}

/// Verifies a device membership certificate against the trusted home root public key.
pub fn verify_membership(
    membership: &DeviceMembershipDto,
    home_root_public_key_b64: &str,
    expected_home_id: &str,
    revocations: &[HomeDeviceRevocationDto],
) -> Result<(), HomeAuthError> {
    if membership.version != 1 {
        return Err(HomeAuthError::UnsupportedVersion(membership.version));
    }
    if membership.home_id != expected_home_id {
        return Err(HomeAuthError::HomeIdMismatch {
            expected: expected_home_id.to_string(),
            got: membership.home_id.clone(),
        });
    }

    // Check key-to-michi_id coherence
    let derived = IdentityManager::derive_michi_id(&membership.device_public_key)
        .map_err(|_| HomeAuthError::InvalidPublicKey)?;
    if derived.to_base64url() != membership.device_michi_id {
        return Err(HomeAuthError::IdentityMismatch);
    }

    // Check revocation
    for rev in revocations {
        if rev.revoked_device_michi_id == membership.device_michi_id {
            return Err(HomeAuthError::DeviceRevoked);
        }
    }

    // Verify signature under home root key
    let root_bytes = decode_base64url_strict(home_root_public_key_b64)
        .map_err(|_| HomeAuthError::InvalidPublicKey)?;
    let root_arr: [u8; 32] = root_bytes
        .try_into()
        .map_err(|_| HomeAuthError::InvalidPublicKey)?;
    let root_vk =
        VerifyingKey::from_bytes(&root_arr).map_err(|_| HomeAuthError::InvalidPublicKey)?;

    let sig_bytes = decode_base64url_strict(&membership.signature)
        .map_err(|_| HomeAuthError::InvalidSignatureEncoding)?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| HomeAuthError::InvalidSignatureEncoding)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

    let canonical_bytes = canonical_membership_bytes(
        &membership.home_id,
        &membership.device_michi_id,
        &membership.device_public_key,
        &membership.device_type,
        &membership.roles,
        &membership.issued_at,
        membership.serial,
    );

    root_vk
        .verify(&canonical_bytes, &sig)
        .map_err(|_| HomeAuthError::InvalidSignature)
}

/// Constructs the domain separation payload for device authentication challenge.
pub fn device_auth_challenge_payload(
    home_id: &str,
    server_michi_id: &str,
    client_michi_id: &str,
    challenge_id: &str,
    challenge_nonce: &str,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(DEVICE_AUTH_DOMAIN_V1);
    buf.extend_from_slice(home_id.as_bytes());
    buf.extend_from_slice(server_michi_id.as_bytes());
    buf.extend_from_slice(client_michi_id.as_bytes());
    buf.extend_from_slice(challenge_id.as_bytes());
    buf.extend_from_slice(challenge_nonce.as_bytes());
    buf
}

/// Signs a device authentication challenge using the client's Ed25519 signing key.
pub fn sign_device_auth_challenge(
    signing_key: &SigningKey,
    home_id: &str,
    server_michi_id: &str,
    client_michi_id: &str,
    challenge_id: &str,
    challenge_nonce: &str,
) -> String {
    let payload = device_auth_challenge_payload(
        home_id,
        server_michi_id,
        client_michi_id,
        challenge_id,
        challenge_nonce,
    );
    encode_base64url(&signing_key.sign(&payload).to_bytes())
}

/// Verifies a client's device authentication challenge signature.
pub fn verify_device_auth_challenge(
    client_public_key_b64: &str,
    signature_b64: &str,
    home_id: &str,
    server_michi_id: &str,
    client_michi_id: &str,
    challenge_id: &str,
    challenge_nonce: &str,
) -> Result<bool, HomeAuthError> {
    let pk_bytes = decode_base64url_strict(client_public_key_b64)
        .map_err(|_| HomeAuthError::InvalidPublicKey)?;
    let pk_arr: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| HomeAuthError::InvalidPublicKey)?;
    let vk = VerifyingKey::from_bytes(&pk_arr).map_err(|_| HomeAuthError::InvalidPublicKey)?;

    let sig_bytes = decode_base64url_strict(signature_b64)
        .map_err(|_| HomeAuthError::InvalidSignatureEncoding)?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| HomeAuthError::InvalidSignatureEncoding)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

    let payload = device_auth_challenge_payload(
        home_id,
        server_michi_id,
        client_michi_id,
        challenge_id,
        challenge_nonce,
    );
    Ok(vk.verify(&payload, &sig).is_ok())
}

/// Constructs the domain separation payload for server mutual authentication confirmation.
pub fn server_auth_confirm_payload(
    home_id: &str,
    server_michi_id: &str,
    client_michi_id: &str,
    challenge_id: &str,
    session_token: &str,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(SERVER_AUTH_DOMAIN_V1);
    buf.extend_from_slice(home_id.as_bytes());
    buf.extend_from_slice(server_michi_id.as_bytes());
    buf.extend_from_slice(client_michi_id.as_bytes());
    buf.extend_from_slice(challenge_id.as_bytes());
    buf.extend_from_slice(session_token.as_bytes());
    buf
}

/// Signs a server mutual session confirmation response using the server's Ed25519 signing key.
pub fn sign_server_auth_confirm(
    signing_key: &SigningKey,
    home_id: &str,
    server_michi_id: &str,
    client_michi_id: &str,
    challenge_id: &str,
    session_token: &str,
) -> String {
    let payload = server_auth_confirm_payload(
        home_id,
        server_michi_id,
        client_michi_id,
        challenge_id,
        session_token,
    );
    encode_base64url(&signing_key.sign(&payload).to_bytes())
}

/// Verifies a server's mutual session confirmation signature.
pub fn verify_server_auth_confirm(
    server_public_key_b64: &str,
    signature_b64: &str,
    home_id: &str,
    server_michi_id: &str,
    client_michi_id: &str,
    challenge_id: &str,
    session_token: &str,
) -> Result<bool, HomeAuthError> {
    let pk_bytes = decode_base64url_strict(server_public_key_b64)
        .map_err(|_| HomeAuthError::InvalidPublicKey)?;
    let pk_arr: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| HomeAuthError::InvalidPublicKey)?;
    let vk = VerifyingKey::from_bytes(&pk_arr).map_err(|_| HomeAuthError::InvalidPublicKey)?;

    let sig_bytes = decode_base64url_strict(signature_b64)
        .map_err(|_| HomeAuthError::InvalidSignatureEncoding)?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| HomeAuthError::InvalidSignatureEncoding)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

    let payload = server_auth_confirm_payload(
        home_id,
        server_michi_id,
        client_michi_id,
        challenge_id,
        session_token,
    );
    Ok(vk.verify(&payload, &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_membership_issue_and_verify_roundtrip() {
        let root = HomeRootAuthority::generate();
        let home_id = root.home_id();

        let client_sk = SigningKey::generate(&mut OsRng);
        let client_vk = client_sk.verifying_key();
        let client_michi_id = MichiId::from_public_key(&client_vk).to_base64url();
        let client_pk_b64 = encode_base64url(&client_vk.to_bytes());

        let membership = root.issue_membership(
            &home_id,
            &client_michi_id,
            &client_pk_b64,
            "stream",
            vec![Role::AudioReceiver],
            "2026-10-04T12:00:00Z",
            1,
        );

        let root_pk_b64 = root.public_key_base64url();
        assert!(verify_membership(&membership, &root_pk_b64, &home_id, &[]).is_ok());
    }

    #[test]
    fn test_membership_rejects_identity_mismatch() {
        let root = HomeRootAuthority::generate();
        let home_id = root.home_id();

        let client_sk = SigningKey::generate(&mut OsRng);
        let other_sk = SigningKey::generate(&mut OsRng);

        let client_vk = client_sk.verifying_key();
        let other_vk = other_sk.verifying_key();

        let wrong_michi_id = MichiId::from_public_key(&other_vk).to_base64url();
        let client_pk_b64 = encode_base64url(&client_vk.to_bytes());

        let membership = root.issue_membership(
            &home_id,
            &wrong_michi_id,
            &client_pk_b64,
            "stream",
            vec![Role::AudioReceiver],
            "2026-10-04T12:00:00Z",
            1,
        );

        let root_pk_b64 = root.public_key_base64url();
        match verify_membership(&membership, &root_pk_b64, &home_id, &[]) {
            Err(HomeAuthError::IdentityMismatch) => (),
            other => panic!("expected IdentityMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_membership_rejects_foreign_home_root() {
        let root1 = HomeRootAuthority::generate();
        let root2 = HomeRootAuthority::generate();
        let home_id = root1.home_id();

        let client_sk = SigningKey::generate(&mut OsRng);
        let client_vk = client_sk.verifying_key();
        let client_michi_id = MichiId::from_public_key(&client_vk).to_base64url();
        let client_pk_b64 = encode_base64url(&client_vk.to_bytes());

        // Signed by root2, verified with root1
        let membership = root2.issue_membership(
            &home_id,
            &client_michi_id,
            &client_pk_b64,
            "stream",
            vec![Role::AudioReceiver],
            "2026-10-04T12:00:00Z",
            1,
        );

        let root1_pk_b64 = root1.public_key_base64url();
        match verify_membership(&membership, &root1_pk_b64, &home_id, &[]) {
            Err(HomeAuthError::InvalidSignature) => (),
            other => panic!("expected InvalidSignature, got {:?}", other),
        }
    }

    #[test]
    fn test_membership_rejects_revoked_device() {
        let root = HomeRootAuthority::generate();
        let home_id = root.home_id();

        let client_sk = SigningKey::generate(&mut OsRng);
        let client_vk = client_sk.verifying_key();
        let client_michi_id = MichiId::from_public_key(&client_vk).to_base64url();
        let client_pk_b64 = encode_base64url(&client_vk.to_bytes());

        let membership = root.issue_membership(
            &home_id,
            &client_michi_id,
            &client_pk_b64,
            "stream",
            vec![Role::AudioReceiver],
            "2026-10-04T12:00:00Z",
            1,
        );

        let rev = root.issue_revocation(
            &home_id,
            &client_michi_id,
            "2026-10-04T12:30:00Z",
            "Lost device",
        );

        let root_pk_b64 = root.public_key_base64url();
        match verify_membership(&membership, &root_pk_b64, &home_id, &[rev]) {
            Err(HomeAuthError::DeviceRevoked) => (),
            other => panic!("expected DeviceRevoked, got {:?}", other),
        }
    }

    #[test]
    fn test_device_auth_challenge_domain_separation_and_replay_protection() {
        let client_sk = SigningKey::generate(&mut OsRng);
        let client_vk = client_sk.verifying_key();
        let client_pk_b64 = encode_base64url(&client_vk.to_bytes());
        let client_michi_id = MichiId::from_public_key(&client_vk).to_base64url();

        let home_id = "vLzV3iL3x7eJ8qZ0a1b2c3d4e5f6g7h8i9j0k1l2m3n";
        let server_michi_id = "97ryPKOLZ-JgVKQFc2ZuuSk0alWzxagdNILuDW26jEc";
        let challenge_id = "550e8400-e29b-41d4-a716-446655440001";
        let challenge_nonce = "VFfZjzw8JeAM7-RFiTSrMA";

        let sig = sign_device_auth_challenge(
            &client_sk,
            home_id,
            server_michi_id,
            &client_michi_id,
            challenge_id,
            challenge_nonce,
        );

        // Valid signature verifies
        assert!(verify_device_auth_challenge(
            &client_pk_b64,
            &sig,
            home_id,
            server_michi_id,
            &client_michi_id,
            challenge_id,
            challenge_nonce,
        )
        .unwrap());

        // Replay with altered nonce fails
        assert!(!verify_device_auth_challenge(
            &client_pk_b64,
            &sig,
            home_id,
            server_michi_id,
            &client_michi_id,
            challenge_id,
            "VFfZjzw8JeAM7-RFiTSrMB", // altered
        )
        .unwrap());

        // Replay with altered challenge_id fails
        assert!(!verify_device_auth_challenge(
            &client_pk_b64,
            &sig,
            home_id,
            server_michi_id,
            &client_michi_id,
            "550e8400-e29b-41d4-a716-446655440002", // altered
            challenge_nonce,
        )
        .unwrap());

        // Replay on different server fails
        assert!(!verify_device_auth_challenge(
            &client_pk_b64,
            &sig,
            home_id,
            "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays", // different server
            &client_michi_id,
            challenge_id,
            challenge_nonce,
        )
        .unwrap());
    }

    #[test]
    fn test_server_mutual_auth_confirmation() {
        let server_sk = SigningKey::generate(&mut OsRng);
        let server_vk = server_sk.verifying_key();
        let server_pk_b64 = encode_base64url(&server_vk.to_bytes());
        let server_michi_id = MichiId::from_public_key(&server_vk).to_base64url();

        let home_id = "vLzV3iL3x7eJ8qZ0a1b2c3d4e5f6g7h8i9j0k1l2m3n";
        let client_michi_id = "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays";
        let challenge_id = "550e8400-e29b-41d4-a716-446655440001";
        let session_token = "michi_ram_token_abc123";

        let sig = sign_server_auth_confirm(
            &server_sk,
            home_id,
            &server_michi_id,
            client_michi_id,
            challenge_id,
            session_token,
        );

        assert!(verify_server_auth_confirm(
            &server_pk_b64,
            &sig,
            home_id,
            &server_michi_id,
            client_michi_id,
            challenge_id,
            session_token,
        )
        .unwrap());

        // Altered token fails
        assert!(!verify_server_auth_confirm(
            &server_pk_b64,
            &sig,
            home_id,
            &server_michi_id,
            client_michi_id,
            challenge_id,
            "michi_ram_token_altered",
        )
        .unwrap());
    }
}
