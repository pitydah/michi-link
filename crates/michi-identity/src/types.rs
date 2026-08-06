//! Canonical contract types for the Michi Link protocol.
//!
//! Single source of truth for service names, roles, API versions, identity
//! representation (Ed25519 + BLAKE3, base64url), announce profiles and the
//! unified pairing DTOs.

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use uuid::Uuid;

/// Canonical service identifiers (contract v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Service {
    #[serde(rename = "michi-music-player")]
    MusicPlayer,
    #[serde(rename = "michi-micro-server")]
    MicroServer,
    #[serde(rename = "michi-mobile")]
    Mobile,
    #[serde(rename = "michi-stream-standard")]
    StreamStandard,
    #[serde(rename = "michi-stream-hifi")]
    StreamHiFi,
}

impl Service {
    pub fn as_str(&self) -> &'static str {
        match self {
            Service::MusicPlayer => "michi-music-player",
            Service::MicroServer => "michi-micro-server",
            Service::Mobile => "michi-mobile",
            Service::StreamStandard => "michi-stream-standard",
            Service::StreamHiFi => "michi-stream-hifi",
        }
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical device roles (contract v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "desktop_player")]
    DesktopPlayer,
    #[serde(rename = "library_master")]
    LibraryMaster,
    #[serde(rename = "sync_host")]
    SyncHost,
    #[serde(rename = "music_server")]
    MusicServer,
    #[serde(rename = "library_host")]
    LibraryHost,
    #[serde(rename = "playback_host")]
    PlaybackHost,
    #[serde(rename = "mobile_player")]
    MobilePlayer,
    #[serde(rename = "remote_controller")]
    RemoteController,
    #[serde(rename = "sync_client")]
    SyncClient,
    #[serde(rename = "audio_receiver")]
    AudioReceiver,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::DesktopPlayer => "desktop_player",
            Role::LibraryMaster => "library_master",
            Role::SyncHost => "sync_host",
            Role::MusicServer => "music_server",
            Role::LibraryHost => "library_host",
            Role::PlaybackHost => "playback_host",
            Role::MobilePlayer => "mobile_player",
            Role::RemoteController => "remote_controller",
            Role::SyncClient => "sync_client",
            Role::AudioReceiver => "audio_receiver",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical API contract versions. Only "v1" and "v1-lite" are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiVersion {
    #[serde(rename = "v1")]
    V1,
    #[serde(rename = "v1-lite")]
    V1Lite,
}

impl ApiVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "v1",
            ApiVersion::V1Lite => "v1-lite",
        }
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Authentication strategies negotiated during pairing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthStrategy {
    #[serde(rename = "PLAYER_PASSWORD")]
    PlayerPassword,
    #[serde(rename = "SERVER_CODE")]
    ServerCode,
    #[serde(rename = "ED25519_CHALLENGE")]
    Ed25519Challenge,
    #[serde(rename = "RECEIVER_BUTTON")]
    ReceiverButton,
    #[serde(rename = "LEGACY")]
    Legacy,
}

/// MichiId: BLAKE3 hash of the raw 32-byte Ed25519 public key.
///
/// The canonical wire representation is base64url without padding (43 chars).
/// Hex is NOT a valid representation in contract v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MichiId(#[serde(with = "michi_id_serde")] pub Option<[u8; 32]>);

impl MichiId {
    /// Derives the identity from an Ed25519 verifying key: blake3(pk raw bytes).
    pub fn from_public_key(pk: &VerifyingKey) -> Self {
        let hash = blake3::hash(&pk.to_bytes());
        Self(Some(*hash.as_bytes()))
    }

    pub fn null() -> Self {
        Self(None)
    }

    pub fn is_present(&self) -> bool {
        self.0.is_some()
    }

    /// Returns the raw 32 bytes of the hash, or None if null.
    pub fn as_bytes(&self) -> Option<&[u8; 32]> {
        self.0.as_ref()
    }

    /// Returns the base64url (no padding) representation, or "null".
    pub fn to_base64url(&self) -> String {
        match self.0 {
            Some(ref b) => encode_base64url(b),
            None => "null".to_string(),
        }
    }

    /// Parses a base64url (no padding) representation of a 32-byte hash.
    pub fn from_base64url(s: &str) -> Option<Self> {
        if s == "null" {
            return Some(Self(None));
        }
        let bytes = decode_base64url_strict(s).ok()?;
        if bytes.len() != 32 {
            return None;
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Some(Self(Some(arr)))
    }
}

impl fmt::Display for MichiId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base64url())
    }
}

impl From<&VerifyingKey> for MichiId {
    fn from(pk: &VerifyingKey) -> Self {
        Self::from_public_key(pk)
    }
}

mod michi_id_serde {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(opt: &Option<[u8; 32]>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt {
            Some(ref b) => s.serialize_str(&super::encode_base64url(b)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<[u8; 32]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(d)?;
        match s {
            Some(ref b64) if b64 == "null" => Ok(None),
            Some(ref b64) => {
                let bytes = super::decode_base64url_strict(b64).map_err(D::Error::custom)?;
                if bytes.len() != 32 {
                    return Err(D::Error::custom(
                        "michi_id must be 32 bytes (43 base64url chars)",
                    ));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(Some(arr))
            }
            None => Ok(None),
        }
    }
}

/// Base64URL without padding — the ONLY wire encoding of contract v1.
pub fn encode_base64url(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Strict Base64URL without padding. Rejects standard alphabet and padding.
pub fn decode_base64url_strict(data: &str) -> Result<Vec<u8>, IdentityErrorLite> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|_| IdentityErrorLite)
}

/// Lightweight error for decode helpers (avoids crate error cycles).
#[derive(Debug, Clone, Copy)]
pub struct IdentityErrorLite;

impl fmt::Display for IdentityErrorLite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid base64url")
    }
}

/// Discovery announce payload (contract v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announce {
    /// Stable device identifier, persisted across restarts.
    pub device_id: String,
    /// Human-readable device name.
    pub name: String,
    pub service: Service,
    pub roles: Vec<Role>,
    pub api_version: ApiVersion,
    pub host: String,
    pub port: u16,
    /// Feature flags. Every value MUST be a boolean.
    pub features: BTreeMap<String, bool>,

    // --- Identity group (all-or-nothing) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub michi_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<i64>,
    /// Nonce as base64url (>= 16 raw bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

impl Announce {
    /// True when the announce carries the complete identity group.
    pub fn is_signed(&self) -> bool {
        matches!(
            (
                &self.michi_id,
                &self.public_key,
                &self.signature,
                &self.timestamp_ms,
                &self.nonce
            ),
            (Some(_), Some(_), Some(_), Some(_), Some(_))
        )
    }

    /// True when only part of the identity group is present (must be rejected).
    pub fn is_partially_signed(&self) -> bool {
        let present = [
            self.michi_id.is_some(),
            self.public_key.is_some(),
            self.signature.is_some(),
            self.timestamp_ms.is_some(),
            self.nonce.is_some(),
        ];
        let count = present.iter().filter(|b| **b).count();
        count > 0 && count < 5
    }
}

/// Service profile used to build a signed announce (contract v1).
///
/// The profile must satisfy the canonical per-service invariants; the
/// engine validates them before signing (`ContractViolation` otherwise).
#[derive(Debug, Clone)]
pub struct AnnounceProfile {
    pub device_id: String,
    pub name: String,
    pub service: Service,
    pub api_version: ApiVersion,
    pub roles: Vec<Role>,
    pub host: String,
    pub port: u16,
    pub features: BTreeMap<String, bool>,
}

/// Trust classification of a discovered peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustLevel {
    /// Valid Ed25519 signature over the canonical payload; carries the peer michi_id.
    Verified(String),
    /// No signature present (legacy announce). Accepted but never trusted as identity.
    Untrusted(String),
    /// Signature present but invalid, inconsistent or stale.
    Invalid,
}

/// Pairing session (contract v1 pairing rules).
///
/// Security invariants:
/// - 5 minutes maximum lifetime (`expires_at`).
/// - 5 maximum attempts (`attempts_remaining`).
/// - Single use (`consumed`).
/// - PIN verifier is keyed with a server-side random secret, so the 6-digit
///   PIN cannot be brute-forced offline from anything exposed to the client.
#[derive(Debug, Clone)]
pub struct PairingSession {
    pub session_id: Uuid,
    pub server_michi_id: String,
    pub server_public_key: Vec<u8>,
    pub client_michi_id: Option<String>,
    pub client_public_key: Vec<u8>,
    /// blake3(server_secret || pin) — never exposed.
    pub pin_verifier: [u8; 32],
    /// Server-side random secret, held in memory only, never serialized.
    pub server_secret: [u8; 32],
    pub created_at: std::time::SystemTime,
    pub expires_at: std::time::SystemTime,
    pub attempts_remaining: u8,
    pub consumed: bool,
    /// Origin key of the pairing client (e.g. source IP) for rate limiting.
    pub source_key: String,
    /// Timestamp of the last confirm attempt.
    pub last_attempt_at: std::time::SystemTime,
}

impl PairingSession {
    pub fn is_expired(&self, now: std::time::SystemTime) -> bool {
        now >= self.expires_at
    }
}

/// POST /pair/start request (contract v1, snake_case only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairStartRequest {
    pub device_name: String,
    pub device_type: String,
    pub roles: Vec<Role>,
    pub auth_strategy: AuthStrategy,
    pub michi_id: String,
    pub public_key: String,
    /// Base64url, >= 16 raw bytes.
    pub challenge_nonce: String,
    /// Ed25519 signature over the raw nonce bytes, base64url (86 chars).
    pub challenge_signature: String,
}

/// POST /pair/start response (contract v1).
///
/// The PIN is displayed locally on the server and NEVER returned over the
/// network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairStartResponse {
    pub session_id: Uuid,
    pub expires_at: String,
    pub attempts_remaining: u8,
    pub server_michi_id: String,
    pub server_public_key: String,
}

/// POST /pair/confirm request (contract v1, snake_case only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairConfirmRequest {
    pub session_id: Uuid,
    pub pin: String,
    pub michi_id: String,
    pub public_key: String,
}

/// POST /pair/confirm response (contract v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairConfirmResponse {
    /// Opaque bearer token issued by the server.
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    /// Stable device id of the paired client (from the server's perspective).
    pub device_id: String,
    /// Stable server id.
    pub server_id: String,
}

/// Identity document embedded in `server/info` (contract v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityDocument {
    pub michi_id: MichiId,
    /// Ed25519 public key in base64url (43 chars).
    pub public_key: String,
    /// Canonical identity scheme: "ed25519-blake3-v1".
    pub identity_scheme: String,
    pub device_name: String,
    pub created_at: String,
}

/// Versioned pairing QR payload (contract v1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingQr {
    /// Always "michi-link-pairing".
    pub format: String,
    /// Always 1 in contract v1.
    pub version: u32,
    pub server_michi_id: String,
    pub server_public_key: String,
    pub session_id: Uuid,
    /// RFC 3339 timestamp.
    pub expires_at: String,
    /// HTTP(S) endpoint of the pairing server.
    pub endpoint: url::Url,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::VerifyingKey;

    #[test]
    fn test_michi_id_from_pk() {
        let pk = VerifyingKey::from_bytes(&[0u8; 32]).unwrap();
        let mid = MichiId::from_public_key(&pk);
        assert!(mid.is_present());
        assert_eq!(mid.as_bytes().unwrap().len(), 32);
        let b64 = mid.to_base64url();
        assert_eq!(b64.len(), 43, "base64url of 32 bytes must be 43 chars");
    }

    #[test]
    fn test_michi_id_null() {
        let mid = MichiId::null();
        assert!(!mid.is_present());
        assert_eq!(mid.to_base64url(), "null");
    }

    #[test]
    fn test_michi_id_serde_roundtrip() {
        let pk = VerifyingKey::from_bytes(&[0u8; 32]).unwrap();
        let mid = MichiId::from_public_key(&pk);
        let json = serde_json::to_string(&mid).unwrap();
        assert_eq!(json.len(), 45, "quoted 43-char base64url");
        let des: MichiId = serde_json::from_str(&json).unwrap();
        assert_eq!(des, mid);

        let null_mid = MichiId::null();
        let null_json = serde_json::to_string(&null_mid).unwrap();
        assert_eq!(null_json, "null");
        let des_null: MichiId = serde_json::from_str(&null_json).unwrap();
        assert!(!des_null.is_present());
    }

    #[test]
    fn test_michi_id_from_base64url() {
        let pk = VerifyingKey::from_bytes(&[0u8; 32]).unwrap();
        let mid = MichiId::from_public_key(&pk);
        let b64 = mid.to_base64url();
        let parsed = MichiId::from_base64url(&b64).unwrap();
        assert_eq!(parsed, mid);
        // Hex is not a valid canonical representation.
        assert!(MichiId::from_base64url("ab".repeat(32).as_str()).is_none());
    }

    #[test]
    fn test_wire_encoding_no_padding_or_std_chars() {
        let bytes = [
            0u8, 255, 128, 1, 2, 3, 250, 251, 252, 253, 254, 1, 2, 3, 4, 5,
        ];
        let b64 = encode_base64url(&bytes);
        assert!(
            !b64.contains('+') && !b64.contains('/') && !b64.contains('='),
            "got {}",
            b64
        );
        let decoded = decode_base64url_strict(&b64).unwrap();
        assert_eq!(decoded, bytes);
        // Standard alphabet is rejected by the strict decoder.
        assert!(decode_base64url_strict("AB+/=").is_err());
    }

    #[test]
    fn test_service_serde() {
        let s = Service::StreamHiFi;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"michi-stream-hifi\"");
        let des: Service = serde_json::from_str(&json).unwrap();
        assert_eq!(des, Service::StreamHiFi);
    }

    #[test]
    fn test_role_serde() {
        let r = Role::AudioReceiver;
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"audio_receiver\"");
        let des: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(des, Role::AudioReceiver);
    }

    #[test]
    fn test_api_version_serde() {
        let v = ApiVersion::V1Lite;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"v1-lite\"");
        let des: ApiVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(des, ApiVersion::V1Lite);
    }

    #[test]
    fn test_auth_strategy_serde() {
        let s = AuthStrategy::Ed25519Challenge;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"ED25519_CHALLENGE\"");
        let des: AuthStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(des, AuthStrategy::Ed25519Challenge);
    }

    #[test]
    fn test_announce_partial_signed_detection() {
        let mut a = Announce {
            device_id: "dev-1".into(),
            name: "Player".into(),
            service: Service::MusicPlayer,
            roles: vec![Role::DesktopPlayer],
            api_version: ApiVersion::V1,
            host: "192.168.1.10".into(),
            port: 8400,
            features: BTreeMap::new(),
            michi_id: Some("x".into()),
            public_key: None,
            signature: None,
            timestamp_ms: None,
            nonce: None,
        };
        assert!(a.is_partially_signed());
        assert!(!a.is_signed());
        a.public_key = Some("y".into());
        a.signature = Some("z".into());
        a.timestamp_ms = Some(1);
        a.nonce = Some("n".into());
        assert!(a.is_signed());
        assert!(!a.is_partially_signed());
    }

    #[test]
    fn test_pairing_dtos_snake_case_roundtrip() {
        let start = PairStartRequest {
            device_name: "Michi Mobile".into(),
            device_type: "mobile".into(),
            roles: vec![Role::MobilePlayer, Role::RemoteController],
            auth_strategy: AuthStrategy::Ed25519Challenge,
            michi_id: "A".repeat(43),
            public_key: "B".repeat(43),
            challenge_nonce: "C".repeat(22),
            challenge_signature: "D".repeat(86),
        };
        let json = serde_json::to_string(&start).unwrap();
        assert!(json.contains("\"device_name\""));
        assert!(json.contains("\"auth_strategy\""));
        assert!(json.contains("\"challenge_nonce\""));
        assert!(!json.contains("deviceName"));
        assert!(!json.contains("camelCase"));
        let des: PairStartRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(des.michi_id, start.michi_id);
    }

    #[test]
    fn test_pairing_dtos_reject_camel_case() {
        // serde denies unknown fields by default: camelCase aliases fail.
        let json = r#"{"deviceName":"x","device_type":"mobile","roles":["mobile_player"],"auth_strategy":"ED25519_CHALLENGE","michi_id":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","public_key":"BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB","challenge_nonce":"CCCCCCCCCCCCCCCCCCCCCC","challenge_signature":"DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD"}"#;
        let res: Result<PairStartRequest, _> = serde_json::from_str(json);
        assert!(res.is_err(), "camelCase deviceName must be rejected");
    }
}
