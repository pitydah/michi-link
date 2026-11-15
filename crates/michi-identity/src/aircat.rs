//! Shared Michi Link AirCat v1 authorization contract.
//!
//! AirCat owns the original-file transfer, but Home owns issuance. This module
//! cannot create a Home identity or access a Home private key. The issuance
//! method is on `HomeRootAuthority` and uses its existing persistent key.
//! Discovery packets, device aliases and bearer-only sessions grant no access.

use crate::types::{decode_base64url_strict, encode_base64url, MichiId};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_FIELD_BYTES: usize = 4096;
const MAX_LIFETIME_MS: i64 = 600_000;
const ALLOWED_SCOPES: [&str; 2] = ["content:read", "content:transfer"];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AirCatGrantError {
    #[error("invalid AirCat v1 grant fields or canonical encoding")]
    InvalidFields,
    #[error("grant belongs to a different Home or authority")]
    WrongHome,
    #[error("grant audience differs from authenticated device")]
    WrongAudience,
    #[error("grant expired or time window invalid")]
    Expired,
    #[error("invalid Home signature")]
    InvalidSignature,
    #[error("Home member revoked")]
    Revoked,
}

/// Approved claims. Callers MUST first authenticate their member, check Home
/// membership/revocations, and validate every selected original file and ACL.
/// This type is intentionally NOT a token, and cannot self-sign.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AirCatGrantClaims {
    pub version: i32,
    pub home_id: String,
    pub authority_michi_id: String,
    pub transfer_id: String,
    pub source_michi_id: String,
    pub destination_michi_id: String,
    pub source_library_id: String,
    pub destination_library_id: String,
    pub selection_scope: String,
    pub scopes: Vec<String>,
    pub manifest_sha256: String,
    pub issued_at_millis: i64,
    pub expires_at_millis: i64,
    pub nonce_base64_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AirCatSignedGrantV1 {
    pub version: i32,
    pub home_id: String,
    pub authority_michi_id: String,
    pub transfer_id: String,
    pub source_michi_id: String,
    pub destination_michi_id: String,
    pub source_library_id: String,
    pub destination_library_id: String,
    pub selection_scope: String,
    pub scopes: Vec<String>,
    pub manifest_sha256: String,
    pub issued_at_millis: i64,
    pub expires_at_millis: i64,
    pub nonce_base64_url: String,
    pub signature_base64_url: String,
}

impl AirCatSignedGrantV1 {
    pub fn claims(&self) -> AirCatGrantClaims {
        AirCatGrantClaims {
            version: self.version,
            home_id: self.home_id.clone(),
            authority_michi_id: self.authority_michi_id.clone(),
            transfer_id: self.transfer_id.clone(),
            source_michi_id: self.source_michi_id.clone(),
            destination_michi_id: self.destination_michi_id.clone(),
            source_library_id: self.source_library_id.clone(),
            destination_library_id: self.destination_library_id.clone(),
            selection_scope: self.selection_scope.clone(),
            scopes: self.scopes.clone(),
            manifest_sha256: self.manifest_sha256.clone(),
            issued_at_millis: self.issued_at_millis,
            expires_at_millis: self.expires_at_millis,
            nonce_base64_url: self.nonce_base64_url.clone(),
        }
    }
}

impl From<AirCatGrantClaims> for AirCatSignedGrantV1 {
    fn from(c: AirCatGrantClaims) -> Self {
        Self {
            version: c.version,
            home_id: c.home_id,
            authority_michi_id: c.authority_michi_id,
            transfer_id: c.transfer_id,
            source_michi_id: c.source_michi_id,
            destination_michi_id: c.destination_michi_id,
            source_library_id: c.source_library_id,
            destination_library_id: c.destination_library_id,
            selection_scope: c.selection_scope,
            scopes: c.scopes,
            manifest_sha256: c.manifest_sha256,
            issued_at_millis: c.issued_at_millis,
            expires_at_millis: c.expires_at_millis,
            nonce_base64_url: c.nonce_base64_url,
            signature_base64_url: String::new(),
        }
    }
}

fn write_field(out: &mut Vec<u8>, s: &str) -> Result<(), AirCatGrantError> {
    let b = s.as_bytes();
    if b.len() > MAX_FIELD_BYTES {
        return Err(AirCatGrantError::InvalidFields);
    }
    out.extend_from_slice(&(b.len() as u32).to_be_bytes());
    out.extend_from_slice(b);
    Ok(())
}

fn valid_michi_id(id: &str) -> bool {
    id.len() == 43
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

pub fn validate_aircat_claims_v1(
    c: &AirCatGrantClaims,
    now_ms: i64,
) -> Result<(), AirCatGrantError> {
    if c.version != 1
        || now_ms <= 0
        || !valid_michi_id(&c.home_id)
        || !valid_michi_id(&c.authority_michi_id)
        || !valid_michi_id(&c.source_michi_id)
        || !valid_michi_id(&c.destination_michi_id)
        || c.home_id != c.authority_michi_id
        || c.transfer_id.is_empty()
        || c.source_library_id.is_empty()
        || c.destination_library_id.is_empty()
        || c.selection_scope.is_empty()
        || c.scopes.len() != 2
        || c.scopes[0] != ALLOWED_SCOPES[0]
        || c.scopes[1] != ALLOWED_SCOPES[1]
        || c.manifest_sha256.len() != 64
        || !c
            .manifest_sha256
            .bytes()
            .all(|x| x.is_ascii_hexdigit() && !x.is_ascii_uppercase())
    {
        return Err(AirCatGrantError::InvalidFields);
    }
    let nonce = decode_base64url_strict(&c.nonce_base64_url)
        .map_err(|_| AirCatGrantError::InvalidFields)?;
    if !(16..=64).contains(&nonce.len()) || encode_base64url(&nonce) != c.nonce_base64_url {
        return Err(AirCatGrantError::InvalidFields);
    }
    let future = now_ms
        .checked_add(60_000)
        .ok_or(AirCatGrantError::Expired)?;
    let life = c
        .expires_at_millis
        .checked_sub(c.issued_at_millis)
        .ok_or(AirCatGrantError::Expired)?;
    if c.issued_at_millis <= 0
        || c.issued_at_millis > future
        || life <= 0
        || life > MAX_LIFETIME_MS
        || now_ms >= c.expires_at_millis
    {
        return Err(AirCatGrantError::Expired);
    }
    Ok(())
}

/// Identical to Android AirCatCanonicalWire.signedGrantBytes: UTF-8 strings
/// with u32 BE lengths, 32/64-bit BE integers and explicit domain separator.
pub fn canonical_aircat_grant_bytes_v1(c: &AirCatGrantClaims) -> Result<Vec<u8>, AirCatGrantError> {
    let mut out = Vec::with_capacity(768);
    write_field(&mut out, "MICHI-AIRCAT-GRANT-V1")?;
    out.extend_from_slice(&c.version.to_be_bytes());
    for item in [
        &c.home_id,
        &c.authority_michi_id,
        &c.transfer_id,
        &c.source_michi_id,
        &c.destination_michi_id,
        &c.source_library_id,
        &c.destination_library_id,
        &c.selection_scope,
    ] {
        write_field(&mut out, item)?;
    }
    out.extend_from_slice(&(c.scopes.len() as i32).to_be_bytes());
    for scope in &c.scopes {
        write_field(&mut out, scope)?;
    }
    write_field(&mut out, &c.manifest_sha256)?;
    out.extend_from_slice(&c.issued_at_millis.to_be_bytes());
    out.extend_from_slice(&c.expires_at_millis.to_be_bytes());
    write_field(&mut out, &c.nonce_base64_url)?;
    Ok(out)
}

/// Verifies ONLY against an authenticated/pinned Home root public key. Producer
/// must additionally check membership, ACLs, original-file version and replay.
pub fn verify_aircat_grant_v1(
    grant: &AirCatSignedGrantV1,
    trusted_root: &VerifyingKey,
    expected_destination: &str,
    now_ms: i64,
    is_revoked: bool,
) -> Result<AirCatGrantClaims, AirCatGrantError> {
    if is_revoked {
        return Err(AirCatGrantError::Revoked);
    }
    let claims = grant.claims();
    validate_aircat_claims_v1(&claims, now_ms)?;
    let root_id = MichiId::from_public_key(trusted_root).to_base64url();
    if claims.home_id != root_id || claims.authority_michi_id != root_id {
        return Err(AirCatGrantError::WrongHome);
    }
    if claims.destination_michi_id != expected_destination {
        return Err(AirCatGrantError::WrongAudience);
    }
    let raw = decode_base64url_strict(&grant.signature_base64_url)
        .map_err(|_| AirCatGrantError::InvalidSignature)?;
    let b: [u8; 64] = raw
        .try_into()
        .map_err(|_| AirCatGrantError::InvalidSignature)?;
    let signature = Signature::from_bytes(&b);
    trusted_root
        .verify(&canonical_aircat_grant_bytes_v1(&claims)?, &signature)
        .map_err(|_| AirCatGrantError::InvalidSignature)?;
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::home::HomeRootAuthority;

    fn sample(root: &HomeRootAuthority, now: i64) -> AirCatGrantClaims {
        let home_id = root.home_id();
        AirCatGrantClaims {
            version: 1,
            home_id: home_id.clone(),
            authority_michi_id: home_id,
            transfer_id: "tx-test".into(),
            source_michi_id: "A".repeat(43),
            destination_michi_id: "B".repeat(43),
            source_library_id: "origin".into(),
            destination_library_id: "target".into(),
            selection_scope: "ALBUMS".into(),
            scopes: ALLOWED_SCOPES.map(|s| s.to_owned()).to_vec(),
            manifest_sha256: "a".repeat(64),
            issued_at_millis: now,
            expires_at_millis: now + 60_000,
            nonce_base64_url: encode_base64url(&[7u8; 24]),
        }
    }
    #[test]
    fn root_signs_and_verifies_without_exposing_private_key() {
        let root = HomeRootAuthority::generate();
        let claims = sample(&root, 1_000_000);
        let grant = root.issue_aircat_grant_v1(&claims, 1_000_000).unwrap();
        let bytes = decode_base64url_strict(&root.public_key_base64url()).unwrap();
        let key = VerifyingKey::from_bytes(&bytes.try_into().unwrap()).unwrap();
        assert_eq!(
            verify_aircat_grant_v1(&grant, &key, &claims.destination_michi_id, 1_000_001, false)
                .unwrap(),
            claims
        );
    }
    #[test]
    fn refuses_bad_audience_revocation_and_mutated_claims() {
        let root = HomeRootAuthority::generate();
        let claims = sample(&root, 1_000_000);
        let mut grant = root.issue_aircat_grant_v1(&claims, 1_000_000).unwrap();
        let bytes = decode_base64url_strict(&root.public_key_base64url()).unwrap();
        let key = VerifyingKey::from_bytes(&bytes.try_into().unwrap()).unwrap();
        assert_eq!(
            verify_aircat_grant_v1(&grant, &key, "foreign", 1_000_001, false),
            Err(AirCatGrantError::WrongAudience)
        );
        assert_eq!(
            verify_aircat_grant_v1(&grant, &key, &claims.destination_michi_id, 1_000_001, true),
            Err(AirCatGrantError::Revoked)
        );
        grant.manifest_sha256 = "b".repeat(64);
        assert_eq!(
            verify_aircat_grant_v1(&grant, &key, &claims.destination_michi_id, 1_000_001, false),
            Err(AirCatGrantError::InvalidSignature)
        );
    }
    #[test]
    fn refuses_longer_than_600s_and_wrong_root() {
        let root = HomeRootAuthority::generate();
        let mut c = sample(&root, 1_000_000);
        c.expires_at_millis = 1_600_001;
        assert_eq!(
            root.issue_aircat_grant_v1(&c, 1_000_000),
            Err(AirCatGrantError::Expired)
        );
        c.expires_at_millis = 1_060_000;
        let grant = root.issue_aircat_grant_v1(&c, 1_000_000).unwrap();
        let other = HomeRootAuthority::generate();
        let bytes = decode_base64url_strict(&other.public_key_base64url()).unwrap();
        let key = VerifyingKey::from_bytes(&bytes.try_into().unwrap()).unwrap();
        assert_eq!(
            verify_aircat_grant_v1(&grant, &key, &c.destination_michi_id, 1_000_001, false),
            Err(AirCatGrantError::WrongHome)
        );
    }

    #[test]
    fn test_aircat_v1_golden_vector_reproducible() {
        // Deterministic Ed25519 signing key seed (32 bytes)
        let seed = [42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let root = HomeRootAuthority::from_signing_key(signing_key);
        let home_id = root.home_id();

        let source_id = "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays";
        let dest_id = "lhlxbEOUgOykbYw6mg4McXy75xBWBX5HOn8pVbixqbY";
        let nonce = encode_base64url(&[0x11u8; 24]);
        let manifest_hash = "35a8427fef1c46924b17ad142d7da6c230e84b8d8cf9f280a5fcfd43b2f5b6f7";
        let issued_at = 1_731_679_400_000i64;
        let expires_at = 1_731_680_000_000i64;

        let claims = AirCatGrantClaims {
            version: 1,
            home_id: home_id.clone(),
            authority_michi_id: home_id.clone(),
            transfer_id: "transfer-uuid-12345".into(),
            source_michi_id: source_id.into(),
            destination_michi_id: dest_id.into(),
            source_library_id: "lib-source-default".into(),
            destination_library_id: "lib-dest-mobile".into(),
            selection_scope: "ALBUMS".into(),
            scopes: vec!["content:read".into(), "content:transfer".into()],
            manifest_sha256: manifest_hash.into(),
            issued_at_millis: issued_at,
            expires_at_millis: expires_at,
            nonce_base64_url: nonce.clone(),
        };

        // Canonical wire bytes test
        let canonical_bytes = canonical_aircat_grant_bytes_v1(&claims).unwrap();
        assert!(!canonical_bytes.is_empty());
        assert_eq!(&canonical_bytes[0..4], &(21u32).to_be_bytes());
        assert_eq!(&canonical_bytes[4..25], b"MICHI-AIRCAT-GRANT-V1");

        // Issue grant using HomeRootAuthority
        let grant = root.issue_aircat_grant_v1(&claims, issued_at).unwrap();
        assert_eq!(grant.version, 1);
        assert_eq!(grant.home_id, home_id);
        assert_eq!(grant.nonce_base64_url, nonce);
        assert_eq!(grant.signature_base64_url.len(), 86);

        // Positive verification
        let root_pk_bytes = decode_base64url_strict(&root.public_key_base64url()).unwrap();
        let verifying_key = VerifyingKey::from_bytes(&root_pk_bytes.try_into().unwrap()).unwrap();
        let verified =
            verify_aircat_grant_v1(&grant, &verifying_key, dest_id, issued_at + 1000, false)
                .unwrap();
        assert_eq!(verified, claims);

        // Negative adversarial test cases:
        // 1. Wrong audience
        assert_eq!(
            verify_aircat_grant_v1(
                &grant,
                &verifying_key,
                "wrong-audience-43-chars-base64url-00000000",
                issued_at + 1000,
                false
            ),
            Err(AirCatGrantError::WrongAudience)
        );

        // 2. Revoked member
        assert_eq!(
            verify_aircat_grant_v1(&grant, &verifying_key, dest_id, issued_at + 1000, true),
            Err(AirCatGrantError::Revoked)
        );

        // 3. Foreign root / wrong home
        let foreign_root = HomeRootAuthority::generate();
        let foreign_pk_bytes =
            decode_base64url_strict(&foreign_root.public_key_base64url()).unwrap();
        let foreign_verifying_key =
            VerifyingKey::from_bytes(&foreign_pk_bytes.try_into().unwrap()).unwrap();
        assert_eq!(
            verify_aircat_grant_v1(
                &grant,
                &foreign_verifying_key,
                dest_id,
                issued_at + 1000,
                false
            ),
            Err(AirCatGrantError::WrongHome)
        );

        // 4. Expired grant
        assert_eq!(
            verify_aircat_grant_v1(&grant, &verifying_key, dest_id, expires_at + 1, false),
            Err(AirCatGrantError::Expired)
        );

        // 5. Tampered manifest hash
        let mut tampered = grant.clone();
        tampered.manifest_sha256 = "0".repeat(64);
        assert_eq!(
            verify_aircat_grant_v1(&tampered, &verifying_key, dest_id, issued_at + 1000, false),
            Err(AirCatGrantError::InvalidSignature)
        );

        // 6. Tampered nonce
        let mut tampered_nonce = grant.clone();
        tampered_nonce.nonce_base64_url = encode_base64url(&[0x99u8; 24]);
        assert_eq!(
            verify_aircat_grant_v1(
                &tampered_nonce,
                &verifying_key,
                dest_id,
                issued_at + 1000,
                false
            ),
            Err(AirCatGrantError::InvalidSignature)
        );
    }
}
