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
//! This module provides the constants, canonical serialization and
//! verification pipeline. The actual UDP socket layer lives in consumers
//! (Player, Micro Server, ...).
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
use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::IdentityError;
use crate::identity::IdentityManager;
use crate::types::{Announce, TrustLevel};

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

/// Peer discovered on the network.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub announce: Announce,
    pub trust: TrustLevel,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

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
        // Evict entries outside the freshness window.
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

/// Discovery engine (in-memory; network I/O lives in consumers).
pub struct DiscoveryEngine {
    identity: Arc<IdentityManager>,
    tx: tokio_sync_broadcast::Sender<DiscoveredPeer>,
    replay: ReplayCache,
}

// Use std broadcast substitute to keep the crate dependency-free of tokio.
mod tokio_sync_broadcast {
    use std::collections::VecDeque;
    use std::sync::{Arc, Condvar, Mutex};

    #[derive(Debug)]
    struct Inner<T> {
        queue: VecDeque<T>,
    }

    impl<T> Default for Inner<T> {
        fn default() -> Self {
            Self {
                queue: VecDeque::new(),
            }
        }
    }

    pub struct Sender<T>(Arc<(Mutex<Inner<T>>, Condvar)>);
    pub struct Receiver<T>(Arc<(Mutex<Inner<T>>, Condvar)>);

    impl<T: Clone> Sender<T> {
        pub fn subscribe(&self) -> Receiver<T> {
            Receiver(self.0.clone())
        }
        pub fn send(&self, item: T) -> Result<usize, ()> {
            let inner = self.0.clone();
            let mut guard = inner.0.lock().unwrap();
            guard.queue.push_back(item);
            inner.1.notify_all();
            Ok(1)
        }
    }

    impl<T: Clone> Receiver<T> {
        pub fn try_recv(&mut self) -> Result<T, ()> {
            let mut guard = self.0 .0.lock().unwrap();
            match guard.queue.pop_front() {
                Some(item) => Ok(item),
                None => Err(()),
            }
        }
    }

    pub fn channel<T>(_capacity: usize) -> (Sender<T>, Receiver<T>) {
        let shared = Arc::new((Mutex::new(Inner::default()), Condvar::new()));
        (Sender(shared.clone()), Receiver(shared))
    }
}

impl DiscoveryEngine {
    pub fn new(identity: Arc<IdentityManager>) -> Self {
        let (tx, _) = tokio_sync_broadcast::channel(256);
        Self {
            identity,
            tx,
            replay: ReplayCache::default(),
        }
    }

    /// Builds a fully signed announce covering every functional field.
    ///
    /// `device_id` must be a stable identifier persisted across restarts;
    /// it is never regenerated per announce.
    pub fn build_signed_announce(
        &self,
        device_id: &str,
        roles: &[crate::types::Role],
        host: &str,
        port: u16,
        features: &std::collections::BTreeMap<String, bool>,
    ) -> Announce {
        let now_ms = Self::now_ms();
        let nonce: [u8; 16] = rand::random();
        let nonce_b64 = Self::to_b64url(&nonce);
        let michi_id = self.identity.michi_id().to_base64url();
        let public_key = self.identity.public_key_b64();

        let mut announce = Announce {
            device_id: device_id.to_string(),
            name: self.identity.device_name().to_string(),
            service: crate::types::Service::MusicPlayer,
            roles: roles.to_vec(),
            api_version: crate::types::ApiVersion::V1,
            host: host.to_string(),
            port,
            features: features.clone(),
            michi_id: Some(michi_id),
            public_key: Some(public_key),
            signature: None,
            timestamp_ms: Some(now_ms),
            nonce: Some(nonce_b64),
        };
        let canonical = Self::canonical_bytes(&announce);
        let (sig, _) = self.identity.sign_standard(&canonical);
        announce.signature = Some(sig);
        announce
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
        // Unsigned: legacy, accepted but never verified as identity.
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

    /// Returns a receiver for discovered peers.
    pub fn subscribe(&self) -> tokio_sync_broadcast::Receiver<DiscoveredPeer> {
        self.tx.subscribe()
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
        let _ = self.tx.send(peer);
        Ok(trust)
    }

    pub fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn to_b64url(data: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityManager;
    use crate::types::{ApiVersion, Role, Service};
    use base64::Engine;
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
            public_key: Some(identity.public_key_b64()),
            signature: None,
            timestamp_ms: Some(ts_ms),
            nonce: Some(nonce.to_string()),
        };
        let canonical = DiscoveryEngine::canonical_bytes(&a);
        let (sig, _) = identity.sign_standard(&canonical);
        a.signature = Some(sig);
        a
    }

    #[test]
    fn test_canonical_bytes_deterministic() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer, Role::LibraryMaster],
            "192.168.1.10",
            8400,
            &features(),
        );
        // Same logical announce always produces the same canonical bytes.
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
        let a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        let trust = engine.verify_announce(&a, None).unwrap();
        assert!(matches!(trust, TrustLevel::Verified(id) if id == mgr.michi_id().to_base64url()));
    }

    #[test]
    fn test_invalid_signature() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        // Corrupt the signature.
        let sig = a.signature.take().unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&sig)
            .unwrap();
        let mut mutated = bytes.clone();
        mutated[0] ^= 0x01;
        a.signature = Some(base64::engine::general_purpose::STANDARD.encode(mutated));
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_altered_payload_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        // Alter a functional field AFTER signing.
        a.port = 8500;
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_altered_public_key_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        a.public_key = Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string());
        let trust = engine.verify_announce(&a, None).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_inconsistent_michi_id_invalid() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let mut a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        a.michi_id = Some("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string());
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
            DiscoveryEngine::now_ms() + TIMESTAMP_WINDOW_MS + 1,
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
        let a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        assert!(matches!(
            engine.verify_announce(&a, None).unwrap(),
            TrustLevel::Verified(_)
        ));
        // Replaying the exact same announce (same nonce) must be rejected.
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
        let mut a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        // Drop the nonce: the group is now partial and must never verify.
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
        let a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        let src: SocketAddr = "192.168.1.99:53318".parse().unwrap();
        let trust = engine.verify_announce(&a, Some(src)).unwrap();
        assert_eq!(trust, TrustLevel::Invalid);
    }

    #[test]
    fn test_host_coherence_match_verified() {
        let mgr = identity("alice");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::DesktopPlayer],
            "192.168.1.10",
            8400,
            &features(),
        );
        let src: SocketAddr = "192.168.1.10:53318".parse().unwrap();
        let trust = engine.verify_announce(&a, Some(src)).unwrap();
        assert!(matches!(trust, TrustLevel::Verified(_)));
    }

    #[test]
    fn test_ingest_announce_verified() {
        let mgr = identity("carol");
        let engine = DiscoveryEngine::new(mgr.clone());
        let a = engine.build_signed_announce(
            DEVICE_ID,
            &[Role::SyncHost],
            "192.168.1.30",
            8500,
            &features(),
        );
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
}
