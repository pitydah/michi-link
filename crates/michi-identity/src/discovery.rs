//! DiscoveryEngine: announcement and discovery with signed Ed25519 announces.
//!
//! ## Network contract (v1)
//!
//! - UDP multicast group: `224.0.0.167`, port `53318`
//! - Announce interval: 30 seconds
//! - Offline timeout: 90 seconds
//! - Encoding: JSON UTF-8, maximum 8 KiB
//! - mDNS service: `_michi-link._tcp.local`
//!
//! This module provides the constants, canonical serialization, per-service
//! profile validation and the verification pipeline. The actual UDP socket
//! layer lives in consumers (Player, Micro Server, ...).
//!
//! ## Service profiles
//!
//! `build_signed_announce` validates the `AnnounceProfile` against the
//! canonical per-service invariants before signing:
//!
//! | Service | api_version | Roles |
//! |---|---|---|
//! | michi-music-player | v1 | desktop_player, library_master, sync_host |
//! | michi-micro-server | v1 | music_server, library_host, playback_host |
//! | michi-mobile | v1 | mobile_player, remote_controller, sync_client |
//! | michi-stream-standard | v1-lite | audio_receiver |
//! | michi-stream-hifi | v1-lite | audio_receiver |
//!
//! ## Signed announce verification order
//!
//! 1. Ed25519 signature over the canonical payload
//! 2. `michi_id` must derive from the announced `public_key`
//! 3. Timestamp within a ±90 s window
//! 4. Nonce replay protection
//! 5. Announced host vs datagram source coherence (when available)
//!
//! Unsigned announces are classified `Untrusted` and never verified as identity.

use serde_json::{Map, Value};
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{ContractViolation, IdentityError};
use crate::identity::IdentityManager;
use crate::types::{Announce, AnnounceProfile, ApiVersion, Role, Service, TrustLevel};

/// Canonical UDP multicast group (IPv4).
pub const MULTICAST_GROUP: &str = "224.0.0.167";
/// Canonical UDP port for discovery announces.
pub const MULTICAST_PORT: u16 = 53318;
/// Recommended announce interval.
pub const ANNOUNCE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
/// Time without announces before a peer is considered offline.
pub const OFFLINE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);
/// Maximum recommended announce payload size.
pub const MAX_ANNOUNCE_BYTES: usize = 8 * 1024;
/// Maximum allowed timestamp skew in milliseconds.
pub const TIMESTAMP_WINDOW_MS: i64 = 90_000;
/// Canonical mDNS service name.
pub const MDNS_SERVICE: &str = "_michi-link._tcp.local";
/// Per-subscriber queue capacity of the in-process fan-out.
pub const SUBSCRIBER_QUEUE_CAPACITY: usize = 256;

/// Peer discovered on the network.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub announce: Announce,
    pub trust: TrustLevel,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Real fan-out broadcast (no shared single queue).
//
// Every subscriber owns an independent bounded queue. `send` delivers a clone
// to every LIVE subscriber; a dropped subscriber is removed lazily. A slow
// subscriber never blocks fast ones: its queue overflows by dropping the
// OLDEST item (each queue is capacity-bounded).
// ---------------------------------------------------------------------------

struct SubInner<T> {
    queue: Mutex<VecDeque<T>>,
    capacity: usize,
}

struct BroadcastState<T> {
    next_id: u64,
    capacity: usize,
    subscribers: HashMap<u64, Weak<SubInner<T>>>,
}

/// Multi-subscriber fan-out sender.
pub struct FanOut<T> {
    state: Arc<Mutex<BroadcastState<T>>>,
}

/// Independent subscriber queue handle.
pub struct Subscriber<T> {
    id: u64,
    inner: Arc<SubInner<T>>,
    state: Arc<Mutex<BroadcastState<T>>>,
}

impl<T> FanOut<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(BroadcastState {
                next_id: 0,
                capacity,
                subscribers: HashMap::new(),
            })),
        }
    }

    pub fn subscribe(&self) -> Subscriber<T> {
        let mut state = self.state.lock().unwrap();
        let id = state.next_id;
        state.next_id += 1;
        let inner = Arc::new(SubInner {
            queue: Mutex::new(VecDeque::new()),
            capacity: state.capacity,
        });
        state.subscribers.insert(id, Arc::downgrade(&inner));
        Subscriber {
            id,
            inner,
            state: self.state.clone(),
        }
    }

    /// Delivers one clone to every live subscriber. Dropped subscribers are
    /// removed; a full queue drops its OLDEST item (overflow policy).
    pub fn send(&self, item: T)
    where
        T: Clone,
    {
        let mut state = self.state.lock().unwrap();
        let mut dead = Vec::new();
        for (id, weak) in state.subscribers.iter() {
            match weak.upgrade() {
                Some(inner) => {
                    let mut queue = inner.queue.lock().unwrap();
                    if queue.len() >= inner.capacity {
                        queue.pop_front();
                    }
                    queue.push_back(item.clone());
                }
                None => dead.push(*id),
            }
        }
        for id in dead {
            state.subscribers.remove(&id);
        }
    }

    /// Number of live subscribers (diagnostics).
    pub fn subscriber_count(&self) -> usize {
        self.state.lock().unwrap().subscribers.len()
    }
}

impl<T: Clone> Subscriber<T> {
    /// Pops the oldest item; `None` when the queue is empty.
    pub fn try_recv(&self) -> Option<T> {
        self.inner.queue.lock().unwrap().pop_front()
    }

    pub fn len(&self) -> usize {
        self.inner.queue.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Drop for Subscriber<T> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.subscribers.remove(&self.id);
        }
    }
}

// ---------------------------------------------------------------------------

/// Bounded replay cache keyed by announced identity and nonce.
#[derive(Debug, Default)]
struct ReplayCache {
    seen: Mutex<VecDeque<(String, String, i64)>>,
}

impl ReplayCache {
    const CAPACITY: usize = 8192;

    /// Returns true when the (michi_id, nonce) pair is a replay.
    fn is_replay(&self, michi_id: &str, nonce: &str, ts_ms: i64, now_ms: i64) -> bool {
        let mut seen = self.seen.lock().unwrap();
        while let Some(front) = seen.front() {
            if now_ms - front.2 > TIMESTAMP_WINDOW_MS {
                seen.pop_front();
            } else {
                break;
            }
        }
        let replay = seen.iter().any(|(id, n, _)| id == michi_id && n == nonce);
        if !replay {
            seen.push_back((michi_id.to_string(), nonce.to_string(), ts_ms));
            while seen.len() > Self::CAPACITY {
                seen.pop_front();
            }
        }
        replay
    }
}

/// Discovery engine (in-memory fan-out; network I/O lives in consumers).
pub struct DiscoveryEngine {
    identity: Arc<IdentityManager>,
    tx: FanOut<DiscoveredPeer>,
    replay: ReplayCache,
}

impl DiscoveryEngine {
    pub fn new(identity: Arc<IdentityManager>) -> Self {
        Self {
            identity,
            tx: FanOut::new(SUBSCRIBER_QUEUE_CAPACITY),
            replay: ReplayCache::default(),
        }
    }

    /// Validates the canonical per-service announce profile.
    pub fn validate_profile(profile: &AnnounceProfile) -> Result<(), IdentityError> {
        if profile.roles.is_empty() {
            return Err(IdentityError::ContractViolation(
                ContractViolation::InvalidRoleProfile,
            ));
        }
        if profile.features.is_empty() {
            return Err(IdentityError::ContractViolation(
                ContractViolation::InvalidServiceProfile,
            ));
        }
        match profile.service {
            Service::MusicPlayer | Service::MicroServer | Service::Mobile => {
                if profile.api_version != ApiVersion::V1 {
                    return Err(IdentityError::ContractViolation(
                        ContractViolation::InvalidApiVersionProfile,
                    ));
                }
            }
            Service::StreamStandard | Service::StreamHiFi => {
                if profile.api_version != ApiVersion::V1Lite {
                    return Err(IdentityError::ContractViolation(
                        ContractViolation::InvalidApiVersionProfile,
                    ));
                }
            }
        }
        let allowed: &[Role] = match profile.service {
            Service::MusicPlayer => &[Role::DesktopPlayer, Role::LibraryMaster, Role::SyncHost],
            Service::MicroServer => &[Role::MusicServer, Role::LibraryHost, Role::PlaybackHost],
            Service::Mobile => &[Role::MobilePlayer, Role::RemoteController, Role::SyncClient],
            Service::StreamStandard | Service::StreamHiFi => &[Role::AudioReceiver],
        };
        // Stream receivers must declare exactly [audio_receiver].
        if matches!(
            profile.service,
            Service::StreamStandard | Service::StreamHiFi
        ) && profile.roles != vec![Role::AudioReceiver]
        {
            return Err(IdentityError::ContractViolation(
                ContractViolation::InvalidRoleProfile,
            ));
        }
        for role in &profile.roles {
            if !allowed.contains(role) {
                return Err(IdentityError::ContractViolation(
                    ContractViolation::InvalidRoleProfile,
                ));
            }
        }
        Ok(())
    }

    /// Builds a fully signed announce for the given service profile.
    ///
    /// `device_id` comes from the profile and must be a stable identifier
    /// persisted across restarts; it is never regenerated per announce.
    pub fn build_signed_announce(
        &self,
        profile: &AnnounceProfile,
    ) -> Result<Announce, IdentityError> {
        Self::validate_profile(profile)?;
        let now_ms = Self::now_ms();
        let nonce: [u8; 16] = rand::random();
        let michi_id = self.identity.michi_id().to_base64url();
        let public_key = self.identity.public_key_base64url();

        let mut announce = Announce {
            device_id: profile.device_id.clone(),
            name: profile.name.clone(),
            service: profile.service,
            roles: profile.roles.clone(),
            api_version: profile.api_version,
            host: profile.host.clone(),
            port: profile.port,
            features: profile.features.clone(),
            michi_id: Some(michi_id),
            public_key: Some(public_key),
            signature: None,
            timestamp_ms: Some(now_ms),
            nonce: Some(crate::types::encode_base64url(&nonce)),
        };
        let canonical = Self::canonical_bytes(&announce);
        let (sig, _) = self.identity.sign_base64url(&canonical);
        announce.signature = Some(sig);
        Ok(announce)
    }

    /// Verifies an announce.
    ///
    /// `source` is the datagram origin when available; a mismatching explicit
    /// IP host marks the announce `Invalid`.
    pub fn verify_announce(
        &self,
        announce: &Announce,
        source: Option<SocketAddr>,
    ) -> Result<TrustLevel, IdentityError> {
        self.verify_announce_at(announce, source, Self::now_ms())
    }

    /// Same as `verify_announce` with an injectable clock (tests).
    pub fn verify_announce_at(
        &self,
        announce: &Announce,
        source: Option<SocketAddr>,
        now_ms: i64,
    ) -> Result<TrustLevel, IdentityError> {
        if !announce.is_signed() {
            if announce.is_partially_signed() {
                return Ok(TrustLevel::Invalid);
            }
            return Ok(TrustLevel::Untrusted(announce.device_id.clone()));
        }

        let michi_id = announce.michi_id.as_deref().unwrap_or_default();
        let public_key = announce.public_key.as_deref().unwrap_or_default();
        let signature = announce.signature.as_deref().unwrap_or_default();
        let timestamp_ms = announce.timestamp_ms.unwrap_or_default();
        let nonce = announce.nonce.as_deref().unwrap_or_default();

        // 1. Ed25519 signature over the canonical payload.
        let canonical = Self::canonical_bytes(announce);
        let valid = IdentityManager::verify(&canonical, signature, public_key)?;
        if !valid {
            return Ok(TrustLevel::Invalid);
        }

        // 2. michi_id must derive from the announced public key.
        let derived = IdentityManager::derive_michi_id(public_key)?;
        if derived.to_base64url() != michi_id {
            return Ok(TrustLevel::Invalid);
        }

        // 3. Timestamp window: not expired, not excessively in the future.
        let delta = now_ms - timestamp_ms;
        if delta.abs() > TIMESTAMP_WINDOW_MS {
            return Err(IdentityError::TimestampOutOfWindow);
        }

        // 4. Replay protection: a nonce is accepted once per identity.
        if self.replay.is_replay(michi_id, nonce, timestamp_ms, now_ms) {
            return Err(IdentityError::ReplayDetected);
        }

        // 5. Host coherence: an explicit IP host must match the datagram source.
        if let Some(src) = source {
            if let Ok(ip) = IpAddr::from_str(announce.host.as_str()) {
                if ip != src.ip() {
                    return Ok(TrustLevel::Invalid);
                }
            }
        }

        Ok(TrustLevel::Verified(michi_id.to_string()))
    }

    /// Canonical deterministic serialization.
    ///
    /// Serializes every functional field (device_id, name, service, roles,
    /// api_version, host, port, features, michi_id, public_key, timestamp_ms,
    /// nonce) as a JSON object with lexicographically ordered keys. The same
    /// logical announce always produces the same bytes.
    pub fn canonical_bytes(announce: &Announce) -> Vec<u8> {
        let mut map = Map::new();
        map.insert(
            "api_version".into(),
            Value::String(announce.api_version.as_str().into()),
        );
        map.insert(
            "device_id".into(),
            Value::String(announce.device_id.clone()),
        );
        map.insert(
            "features".into(),
            serde_json::to_value(&announce.features).unwrap_or(Value::Null),
        );
        map.insert("host".into(), Value::String(announce.host.clone()));
        if let Some(michi_id) = &announce.michi_id {
            map.insert("michi_id".into(), Value::String(michi_id.clone()));
        }
        map.insert("name".into(), Value::String(announce.name.clone()));
        if let Some(nonce) = &announce.nonce {
            map.insert("nonce".into(), Value::String(nonce.clone()));
        }
        map.insert("port".into(), Value::Number(announce.port.into()));
        if let Some(pk) = &announce.public_key {
            map.insert("public_key".into(), Value::String(pk.clone()));
        }
        map.insert(
            "roles".into(),
            Value::Array(
                announce
                    .roles
                    .iter()
                    .map(|r| Value::String(r.as_str().into()))
                    .collect(),
            ),
        );
        map.insert(
            "service".into(),
            Value::String(announce.service.as_str().into()),
        );
        if let Some(ts) = announce.timestamp_ms {
            map.insert("timestamp_ms".into(), Value::Number(ts.into()));
        }
        serde_json::to_vec(&map).unwrap_or_default()
    }

    /// Returns a subscriber for discovered peers (independent queue).
    pub fn subscribe(&self) -> Subscriber<DiscoveredPeer> {
        self.tx.subscribe()
    }

    /// Live subscriber count (diagnostics).
    pub fn subscriber_count(&self) -> usize {
        self.tx.subscriber_count()
    }

    /// Simulates the reception of an announce (for tests without network).
    pub fn ingest_announce(
        &self,
        announce: Announce,
        source: Option<SocketAddr>,
    ) -> Result<TrustLevel, IdentityError> {
        let trust = self.verify_announce(&announce, source)?;
        let peer = DiscoveredPeer {
            announce,
            trust: trust.clone(),
            last_seen: chrono::Utc::now(),
        };
        self.tx.send(peer);
        Ok(trust)
    }

    pub fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityManager;
    use crate::types::encode_base64url;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    const DEVICE_ID: &str = "stable-device-01";

    fn identity(name: &str) -> Arc<IdentityManager> {
        let dir = TempDir::new().unwrap();
        Arc::new(IdentityManager::generate(dir.path(), name, "test-password").unwrap())
    }

    fn features() -> BTreeMap<String, bool> {
        let mut f = BTreeMap::new();
        f.insert("library".to_string(), true);
        f.insert("events".to_string(), false);
        f
    }

    fn player_profile() -> AnnounceProfile {
        AnnounceProfile {
            device_id: DEVICE_ID.into(),
            name: "Player".into(),
            service: Service::MusicPlayer,
            api_version: ApiVersion::V1,
            roles: vec![Role::DesktopPlayer, Role::LibraryMaster],
            host: "192.168.1.10".into(),
            port: 8400,
            features: features(),
        }
    }

    fn micro_profile() -> AnnounceProfile {
        AnnounceProfile {
            device_id: "micro-01".into(),
            name: "Micro Server".into(),
            service: Service::MicroServer,
            api_version: ApiVersion::V1,
            roles: vec![Role::MusicServer, Role::LibraryHost],
            host: "192.168.1.20".into(),
            port: 8500,
            features: features(),
        }
    }

    fn mobile_profile() -> AnnounceProfile {
        AnnounceProfile {
            device_id: "mobile-01".into(),
            name: "Mobile".into(),
            service: Service::Mobile,
            api_version: ApiVersion::V1,
            roles: vec![Role::MobilePlayer, Role::RemoteController],
            host: "192.168.1.30".into(),
            port: 8400,
            features: features(),
        }
    }

    fn stream_profile(hifi: bool) -> AnnounceProfile {
        AnnounceProfile {
            device_id: "stream-01".into(),
            name: if hifi {
                "Stream Hi-Fi"
            } else {
                "Stream Standard"
            }
            .into(),
            service: if hifi {
                Service::StreamHiFi
            } else {
                Service::StreamStandard
            },
            api_version: ApiVersion::V1Lite,
            roles: vec![Role::AudioReceiver],
            host: "192.168.1.40".into(),
            port: 8600,
            features: {
                let mut f = BTreeMap::new();
                f.insert("session".to_string(), true);
                f.insert("volume".to_string(), true);
                f
            },
        }
    }

    /// Signs an announce over a fully specified payload (custom ts/nonce).
    fn signed_announce_at(identity: &IdentityManager, ts_ms: i64, nonce: &str) -> Announce {
        let mut a = Announce {
            device_id: DEVICE_ID.to_string(),
            name: identity.device_name().to_string(),
            service: Service::MusicPlayer,
            roles: vec![Role::DesktopPlayer],
            api_version: ApiVersion::V1,
            host: "192.168.1.10".into(),
            port: 8400,
            features: features(),
            michi_id: Some(identity.michi_id().to_base64url()),
            public_key: Some(identity.public_key_base64url()),
            signature: None,
            timestamp_ms: Some(ts_ms),
            nonce: Some(nonce.to_string()),
        };
        let canonical = DiscoveryEngine::canonical_bytes(&a);
        let (sig, _) = identity.sign_base64url(&canonical);
        a.signature = Some(sig);
        a
    }

    // --- Service profiles ---

    #[test]
    fn test_player_announce_profile() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        assert_eq!(a.service, Service::MusicPlayer);
        assert_eq!(a.api_version, ApiVersion::V1);
        assert!(matches!(
            engine.verify_announce(&a, None).unwrap(),
            TrustLevel::Verified(_)
        ));
    }

    #[test]
    fn test_micro_announce_profile() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&micro_profile()).unwrap();
        assert_eq!(a.service, Service::MicroServer);
        assert!(matches!(
            engine.verify_announce(&a, None).unwrap(),
            TrustLevel::Verified(_)
        ));
    }

    #[test]
    fn test_mobile_announce_profile() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&mobile_profile()).unwrap();
        assert_eq!(a.service, Service::Mobile);
        assert!(matches!(
            engine.verify_announce(&a, None).unwrap(),
            TrustLevel::Verified(_)
        ));
    }

    #[test]
    fn test_stream_standard_profile() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine
            .build_signed_announce(&stream_profile(false))
            .unwrap();
        assert_eq!(a.service, Service::StreamStandard);
        assert_eq!(a.api_version, ApiVersion::V1Lite);
        assert_eq!(a.roles, vec![Role::AudioReceiver]);
        assert!(matches!(
            engine.verify_announce(&a, None).unwrap(),
            TrustLevel::Verified(_)
        ));
    }

    #[test]
    fn test_stream_hifi_profile() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&stream_profile(true)).unwrap();
        assert_eq!(a.service, Service::StreamHiFi);
        assert_eq!(a.api_version, ApiVersion::V1Lite);
        assert!(matches!(
            engine.verify_announce(&a, None).unwrap(),
            TrustLevel::Verified(_)
        ));
    }

    #[test]
    fn test_reject_player_v1_lite() {
        let mut p = player_profile();
        p.api_version = ApiVersion::V1Lite;
        let err = DiscoveryEngine::validate_profile(&p).unwrap_err();
        assert!(
            matches!(
                err,
                IdentityError::ContractViolation(ContractViolation::InvalidApiVersionProfile)
            ),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_reject_stream_v1() {
        let mut p = stream_profile(false);
        p.api_version = ApiVersion::V1;
        let err = DiscoveryEngine::validate_profile(&p).unwrap_err();
        assert!(
            matches!(
                err,
                IdentityError::ContractViolation(ContractViolation::InvalidApiVersionProfile)
            ),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_reject_invalid_roles() {
        // Mobile cannot be an audio_receiver.
        let mut p = mobile_profile();
        p.roles = vec![Role::AudioReceiver];
        let err = DiscoveryEngine::validate_profile(&p).unwrap_err();
        assert!(
            matches!(
                err,
                IdentityError::ContractViolation(ContractViolation::InvalidRoleProfile)
            ),
            "got {:?}",
            err
        );

        // Micro cannot carry receiver roles.
        let mut p = micro_profile();
        p.roles = vec![Role::MusicServer, Role::AudioReceiver];
        assert!(DiscoveryEngine::validate_profile(&p).is_err());

        // Stream must be exactly [audio_receiver].
        let mut p = stream_profile(true);
        p.roles = vec![Role::AudioReceiver, Role::DesktopPlayer];
        assert!(DiscoveryEngine::validate_profile(&p).is_err());

        // Empty roles are rejected.
        let mut p = player_profile();
        p.roles = vec![];
        assert!(DiscoveryEngine::validate_profile(&p).is_err());

        // Empty features are rejected.
        let mut p = player_profile();
        p.features = BTreeMap::new();
        assert!(DiscoveryEngine::validate_profile(&p).is_err());
    }

    #[test]
    fn test_every_profile_validates_discovery_schema_shape() {
        // Structural contract checks (schema validation happens in JS):
        // canonical bytes must serialize all profiles identically by keys.
        let engine = DiscoveryEngine::new(identity("alice"));
        for profile in [
            player_profile(),
            micro_profile(),
            mobile_profile(),
            stream_profile(false),
            stream_profile(true),
        ] {
            let a = engine.build_signed_announce(&profile).unwrap();
            let json: Value =
                serde_json::from_slice(&DiscoveryEngine::canonical_bytes(&a)).unwrap();
            assert_eq!(json["service"], profile.service.as_str());
            assert_eq!(json["api_version"], profile.api_version.as_str());
            assert!(json["features"].is_object());
            for (k, v) in json["features"].as_object().unwrap() {
                assert!(v.is_boolean(), "feature {} is not boolean", k);
            }
            // Wire fields must be strict base64url of the right length.
            // (signature is intentionally NOT part of the canonical payload.)
            let pk = json["public_key"].as_str().unwrap();
            assert_eq!(pk.len(), 43);
            let mid = json["michi_id"].as_str().unwrap();
            assert_eq!(mid.len(), 43);
            let sig = a.signature.as_ref().unwrap();
            assert_eq!(sig.len(), 86);
        }
    }

    // --- Signature verification pipeline ---

    #[test]
    fn test_canonical_bytes_deterministic() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        let bytes_a = DiscoveryEngine::canonical_bytes(&a);
        let bytes_a2 = DiscoveryEngine::canonical_bytes(&a);
        assert_eq!(bytes_a, bytes_a2);
        let cloned = a.clone();
        assert_eq!(bytes_a, DiscoveryEngine::canonical_bytes(&cloned));
    }

    #[test]
    fn test_valid_signature_verified() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        let trust = engine.verify_announce(&a, None).unwrap();
        assert!(matches!(trust, TrustLevel::Verified(id) if id == mgr.michi_id().to_base64url()));
    }

    #[test]
    fn test_invalid_signature() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(&player_profile()).unwrap();
        let sig = a.signature.take().unwrap();
        let bytes = crate::types::decode_base64url_strict(&sig).unwrap();
        let mut mutated = bytes.clone();
        mutated[0] ^= 0x01;
        a.signature = Some(encode_base64url(&mutated));
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_altered_payload_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(&player_profile()).unwrap();
        a.port = 8500;
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_altered_public_key_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(&player_profile()).unwrap();
        a.public_key = Some("A".repeat(43));
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_inconsistent_michi_id_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(&player_profile()).unwrap();
        a.michi_id = Some("B".repeat(43));
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_expired_timestamp_rejected() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = signed_announce_at(
            &mgr,
            DiscoveryEngine::now_ms() - TIMESTAMP_WINDOW_MS - 1,
            "expired-nonce-01",
        );
        let err = engine.verify_announce(&a, None).unwrap_err();
        assert!(
            matches!(err, IdentityError::TimestampOutOfWindow),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_future_timestamp_rejected() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = signed_announce_at(
            &mgr,
            // 5s margin: sign-to-verify latency must never pull the future
            // timestamp back inside the window (deterministic test).
            DiscoveryEngine::now_ms() + TIMESTAMP_WINDOW_MS + 5000,
            "future-nonce-01",
        );
        let err = engine.verify_announce(&a, None).unwrap_err();
        assert!(
            matches!(err, IdentityError::TimestampOutOfWindow),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_repeated_nonce_rejected() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        assert!(matches!(
            engine.verify_announce(&a, None).unwrap(),
            TrustLevel::Verified(_)
        ));
        let err = engine.verify_announce(&a, None).unwrap_err();
        assert!(
            matches!(err, IdentityError::ReplayDetected),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_partially_signed_announce_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(&player_profile()).unwrap();
        a.nonce = None;
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_unsigned_announce_untrusted() {
        let engine = DiscoveryEngine::new(identity("bob"));
        let a = Announce {
            device_id: DEVICE_ID.to_string(),
            name: "Legacy Device".into(),
            service: Service::MusicPlayer,
            roles: vec![Role::DesktopPlayer],
            api_version: ApiVersion::V1,
            host: "192.168.1.20".into(),
            port: 8400,
            features: features(),
            michi_id: None,
            public_key: None,
            signature: None,
            timestamp_ms: None,
            nonce: None,
        };
        let trust = engine.verify_announce(&a, None).unwrap();
        assert!(matches!(trust, TrustLevel::Untrusted(_)));
    }

    #[test]
    fn test_host_coherence_mismatch_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        let src: SocketAddr = "192.168.1.99:53318".parse().unwrap();
        let trust = engine.verify_announce(&a, Some(src)).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_host_coherence_match_verified() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        let src: SocketAddr = "192.168.1.10:53318".parse().unwrap();
        let trust = engine.verify_announce(&a, Some(src)).unwrap();
        assert!(matches!(trust, TrustLevel::Verified(_)));
    }

    #[test]
    fn test_ingest_announce_verified() {
        let mgr = identity("carol");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(&micro_profile()).unwrap();
        let trust = engine.ingest_announce(a, None).unwrap();
        assert!(matches!(trust, TrustLevel::Verified(_)));
    }

    #[test]
    fn test_constants_are_canonical() {
        assert_eq!(MULTICAST_GROUP, "224.0.0.167");
        assert_eq!(MULTICAST_PORT, 53318);
        assert_eq!(ANNOUNCE_INTERVAL.as_secs(), 30);
        assert_eq!(OFFLINE_TIMEOUT.as_secs(), 90);
        assert_eq!(MAX_ANNOUNCE_BYTES, 8192);
        assert_eq!(MDNS_SERVICE, "_michi-link._tcp.local");
    }

    // --- Fan-out broadcast ---

    #[test]
    fn test_two_subscribers_receive_same_announce() {
        let engine = DiscoveryEngine::new(identity("alice"));
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        let sub1 = engine.subscribe();
        let sub2 = engine.subscribe();
        assert_eq!(engine.subscriber_count(), 2);

        engine.ingest_announce(a.clone(), None).unwrap();
        let peer1 = sub1.try_recv().expect("sub1 must receive the announce");
        let peer2 = sub2.try_recv().expect("sub2 must receive the announce");
        assert_eq!(peer1.announce.device_id, a.device_id);
        assert_eq!(peer2.announce.device_id, a.device_id);
        assert_eq!(peer1.trust, peer2.trust);
    }

    #[test]
    fn test_slow_subscriber_does_not_block_fast_subscriber() {
        let engine = DiscoveryEngine::new(identity("alice"));
        let _a = engine.build_signed_announce(&player_profile()).unwrap();
        let slow = engine.subscribe();
        let fast = engine.subscribe();

        for _ in 0..10 {
            // A fresh announce per ingest: the replay cache rejects repeats.
            let fresh = engine.build_signed_announce(&player_profile()).unwrap();
            engine.ingest_announce(fresh, None).unwrap();
        }
        // Fast subscriber drained everything; slow one is independent.
        assert_eq!(fast.len(), 10);
        assert_eq!(slow.len(), 10);
        fast.try_recv();
        assert_eq!(fast.len(), 9);
        assert_eq!(slow.len(), 10, "slow subscriber is not drained by fast");
    }

    #[test]
    fn test_subscriber_overflow_policy() {
        let engine = DiscoveryEngine::new(identity("alice"));
        let a = engine.build_signed_announce(&player_profile()).unwrap();
        // Create a subscriber with a tiny capacity via the raw fan-out.
        let fan: FanOut<DiscoveredPeer> = FanOut::new(2);
        let sub = fan.subscribe();
        for i in 0..5 {
            let peer = DiscoveredPeer {
                announce: a.clone(),
                trust: TrustLevel::Untrusted(format!("peer-{}", i)),
                last_seen: chrono::Utc::now(),
            };
            fan.send(peer);
        }
        // Overflow drops the OLDEST: only the last 2 remain.
        assert_eq!(sub.len(), 2);
        let first = sub.try_recv().unwrap();
        assert!(matches!(first.trust, TrustLevel::Untrusted(ref id) if id == "peer-3"));
    }

    #[test]
    fn test_dropped_subscriber_is_removed() {
        let fan: FanOut<DiscoveredPeer> = FanOut::new(8);
        let a = Announce {
            device_id: "x".into(),
            name: "n".into(),
            service: Service::MusicPlayer,
            roles: vec![Role::DesktopPlayer],
            api_version: ApiVersion::V1,
            host: "192.168.1.1".into(),
            port: 8400,
            features: features(),
            michi_id: None,
            public_key: None,
            signature: None,
            timestamp_ms: None,
            nonce: None,
        };
        let peer = DiscoveredPeer {
            announce: a,
            trust: TrustLevel::Untrusted("x".into()),
            last_seen: chrono::Utc::now(),
        };
        {
            let _sub = fan.subscribe();
            assert_eq!(fan.subscriber_count(), 1);
            fan.send(peer.clone());
        }
        // Dropped subscriber no longer counts and is removed on next send.
        fan.send(peer);
        assert_eq!(fan.subscriber_count(), 0);
    }
}
