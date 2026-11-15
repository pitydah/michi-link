//! Shared Michi Link AirCat v1 authorization contract.
//!
//! AirCat owns the original-file transfer, but Home owns issuance. This module
//! cannot create a Home identity or access a Home private key. The issuance
//! method is on `HomeRootAuthority` and uses its existing persistent key.
//! Discovery packets, device aliases and bearer-only sessions grant no access.

use crate::types::{decode_base64url_strict, encode_base64url, MichiId};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;

/// Deterministic Ed25519 signing key seed for test fixtures and golden vectors ONLY.
/// Seed byte 0x42 repeated 32 times (decimal 66). NEVER use in production environments.
pub const AIRCAT_GOLDEN_SEED: [u8; 32] = [0x42u8; 32];

const MAX_FIELD_BYTES: usize = 4096;
const MAX_LIFETIME_MS: i64 = 600_000;
const ALLOWED_SCOPES: [&str; 2] = ["content:read", "content:transfer"];

fn default_manifest_version() -> i32 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AirCatSelectionItem {
    pub logical_track_id: String,
    pub remote_track_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub format: String,
    pub size: i64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AirCatSelectionManifest {
    #[serde(default = "default_manifest_version")]
    pub manifest_version: i32,
    pub transfer_id: String,
    pub source_device_id: String,
    pub total_tracks: i32,
    pub total_bytes: i64,
    pub items: Vec<AirCatSelectionItem>,
}

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

pub fn canonical_aircat_manifest_bytes_v1(
    m: &AirCatSelectionManifest,
) -> Result<Vec<u8>, AirCatGrantError> {
    if m.manifest_version != 1
        || m.items.is_empty()
        || m.items.len() > 10_000
        || m.total_tracks as usize != m.items.len()
        || m.total_bytes <= 0
        || m.transfer_id.is_empty()
        || m.source_device_id.is_empty()
    {
        return Err(AirCatGrantError::InvalidFields);
    }
    let mut out = Vec::with_capacity(1024);
    write_field(&mut out, "MICHI-AIRCAT-MANIFEST-V1")?;
    out.extend_from_slice(&m.manifest_version.to_be_bytes());
    write_field(&mut out, &m.transfer_id)?;
    write_field(&mut out, &m.source_device_id)?;
    out.extend_from_slice(&m.total_tracks.to_be_bytes());
    out.extend_from_slice(&m.total_bytes.to_be_bytes());
    out.extend_from_slice(&(m.items.len() as i32).to_be_bytes());
    let mut ids = HashSet::new();
    let mut total = 0i64;
    for f in &m.items {
        if f.size <= 0
            || f.format.is_empty()
            || f.remote_track_id.is_empty()
            || f.logical_track_id.is_empty()
            || !ids.insert(f.logical_track_id.as_str())
            || f.sha256.len() != 64
            || !f.sha256.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(AirCatGrantError::InvalidFields);
        }
        total = total
            .checked_add(f.size)
            .ok_or(AirCatGrantError::InvalidFields)?;
        for field in [
            &f.logical_track_id,
            &f.remote_track_id,
            &f.title,
            &f.artist,
            &f.album,
            &f.format,
        ] {
            write_field(&mut out, field)?;
        }
        out.extend_from_slice(&f.size.to_be_bytes());
        write_field(&mut out, &f.sha256.to_ascii_lowercase())?;
    }
    if total != m.total_bytes {
        return Err(AirCatGrantError::InvalidFields);
    }
    Ok(out)
}

pub fn canonical_aircat_manifest_sha256(
    m: &AirCatSelectionManifest,
) -> Result<String, AirCatGrantError> {
    let bytes = canonical_aircat_manifest_bytes_v1(m)?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

/// Computes the mandatory normative response receipt:
/// `X-Michi-AirCat-Authorized-Grant-SHA256 = lowercase_hex(sha256(ASCII(grant_header_raw)))`
pub fn receipt_for_grant_header(grant_header_raw: &str) -> Result<String, AirCatGrantError> {
    let trimmed = grant_header_raw.trim();
    if trimmed.is_empty()
        || trimmed.len() > 8192
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(AirCatGrantError::InvalidFields);
    }
    Ok(hex::encode(Sha256::digest(trimmed.as_bytes())))
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
        let seed = AIRCAT_GOLDEN_SEED;
        assert_eq!(seed, [0x42u8; 32]);
        assert_eq!(seed[0], 66); // 0x42 is decimal 66

        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let root = HomeRootAuthority::from_signing_key(signing_key);
        let home_id = root.home_id();
        let root_pk_b64 = root.public_key_base64url();

        // Exact assertions against golden values
        assert_eq!(root_pk_b64, "IVL40Zt5HSRFMkLhXy6rbLfP-ntqXtMAl5YOBpiB2xI");
        assert_eq!(home_id, "_vbfpIsHOSTENlOQENeBL75QCWroJWn9rTX3lii8AIQ");
        // Verify Home ID is NOT the public key (BLAKE3-derived)
        assert_ne!(home_id, root_pk_b64);

        // 1. Selection Manifest with 2 real tracks (positive sizes and non-empty SHA-256)
        let manifest = AirCatSelectionManifest {
            manifest_version: 1,
            transfer_id: "transfer-uuid-12345".into(),
            source_device_id: "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays".into(),
            total_tracks: 2,
            total_bytes: 1572864,
            items: vec![
                AirCatSelectionItem {
                    logical_track_id: "logical-track-001".into(),
                    remote_track_id: "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d".into(),
                    title: "Song One".into(),
                    artist: "Artist One".into(),
                    album: "Album One".into(),
                    format: "flac".into(),
                    size: 1048576,
                    sha256: "042d84d3772b8cc9080a8f388c428c11d88f8566e5a38fda322fc8ea33492cfc"
                        .into(),
                },
                AirCatSelectionItem {
                    logical_track_id: "logical-track-002".into(),
                    remote_track_id: "b2c3d4e5-f6a7-4b8c-9d0e-1f2a3b4c5d6e".into(),
                    title: "Song Two".into(),
                    artist: "Artist Two".into(),
                    album: "Album Two".into(),
                    format: "flac".into(),
                    size: 524288,
                    sha256: "4da375e72a048539cac37b0276ef59007bc58b2747ac6a724ace46d0d5f6d328"
                        .into(),
                },
            ],
        };

        let manifest_bytes = canonical_aircat_manifest_bytes_v1(&manifest).unwrap();
        assert_eq!(manifest_bytes.len(), 486);
        let manifest_hash = canonical_aircat_manifest_sha256(&manifest).unwrap();
        assert_eq!(
            manifest_hash,
            "df2221e0e394146f646543f4c824f00c7c3b40e1950d7a42d9db57f84597b066"
        );

        let source_id = "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays";
        let dest_id = "lhlxbEOUgOykbYw6mg4McXy75xBWBX5HOn8pVbixqbY";
        let nonce = "WnBfQ2hhbGxlbmdlTm9uY2VfMTIzNDU2".to_string();
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
            manifest_sha256: manifest_hash.clone(),
            issued_at_millis: issued_at,
            expires_at_millis: expires_at,
            nonce_base64_url: nonce.clone(),
        };

        // Canonical wire bytes test
        let canonical_bytes = canonical_aircat_grant_bytes_v1(&claims).unwrap();
        assert_eq!(canonical_bytes.len(), 451);
        let canonical_hex = hex::encode(&canonical_bytes);
        assert_eq!(
            canonical_hex,
            "000000154d494348492d4149524341542d4752414e542d5631000000010000002b5f766266704973484f5354454e6c4f51454e65424c3735514357726f4a576e39725458336c6969384149510000002b5f766266704973484f5354454e6c4f51454e65424c3735514357726f4a576e39725458336c696938414951000000137472616e736665722d757569642d31323334350000002b516c47516f7351737a4c5173653035374d436177333249414858762d49356b6c6d414173626976496179730000002b6c686c7862454f55674f796b625977366d67344d6358793735784257425835484f6e387056626978716259000000126c69622d736f757263652d64656661756c740000000f6c69622d646573742d6d6f62696c6500000006414c42554d53000000020000000c636f6e74656e743a7265616400000010636f6e74656e743a7472616e736665720000004064663232323165306533393431343666363436353433663463383234663030633763336234306531393530643761343264396462353766383435393762303636000001933022b04000000193302bd80000000020576e4266513268686247786c626d646c546d3975593256664d54497a4e445532"
        );

        // Issue grant using HomeRootAuthority
        let grant = root.issue_aircat_grant_v1(&claims, issued_at).unwrap();
        assert_eq!(grant.version, 1);
        assert_eq!(grant.home_id, home_id);
        assert_eq!(grant.authority_michi_id, home_id);
        assert_eq!(grant.nonce_base64_url, nonce);
        assert_eq!(
            grant.signature_base64_url,
            "ccQtsU8Nzu4zQAQ_WZf_oFCTd07QJJptMDckzbeay1FmgF1oJuAYM_TWgIAXVyOe3kApsJqIoJPQqLa5iIwKBA"
        );

        // Header and receipt test
        let grant_json = serde_json::to_string(&grant).unwrap();
        let grant_header = encode_base64url(grant_json.as_bytes());
        let receipt = receipt_for_grant_header(&grant_header).unwrap();
        assert_eq!(receipt.len(), 64);
        assert_eq!(
            receipt,
            hex::encode(sha2::Sha256::digest(grant_header.as_bytes()))
        );

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

        // 7. Extra scopes
        let mut tampered_scopes = claims.clone();
        tampered_scopes.scopes.push("admin:all".to_string());
        assert!(canonical_aircat_grant_bytes_v1(&tampered_scopes).is_ok());
        assert_eq!(
            validate_aircat_claims_v1(&tampered_scopes, issued_at + 1000),
            Err(AirCatGrantError::InvalidFields)
        );

        // 8. Invalid receipt input (contains spaces or illegal characters)
        assert_eq!(
            receipt_for_grant_header("invalid header with spaces"),
            Err(AirCatGrantError::InvalidFields)
        );
        assert_eq!(
            receipt_for_grant_header(""),
            Err(AirCatGrantError::InvalidFields)
        );
    }
}
