//! Canonical Michi Link v1 capability catalog.
//!
//! Single normative source of truth for feature capability names advertised in
//! `/api/v1/server/info`, `/api/v1/capabilities`, and Whisker/Scent discovery
//! announcements.
//!
//! IMPORTANT: Advertising a capability flag only declares protocol support and
//! runtime readiness; it MUST NEVER be treated as client authorization. Every
//! protected operation requires its own cryptographic session / authorization
//! verification (for example, `AIRCAT_STRICT` requires an active `HOME_MEMBERSHIP`
//! session over pinned HTTPS/TLS plus a signed Home grant).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Canonical identity scheme identifier (Ed25519 + BLAKE3).
pub const IDENTITY_SCHEME_V1: &str = "ed25519-blake3-v1";

/// Core library, search, streaming, download, and synchronization capabilities.
pub const CAP_LIBRARY: &str = "library";
pub const CAP_SEARCH: &str = "search";
pub const CAP_STREAMING: &str = "streaming";
pub const CAP_DOWNLOAD: &str = "download";
pub const CAP_SYNC: &str = "sync";
pub const CAP_SYNC_MANIFEST: &str = "sync_manifest";
pub const CAP_ARTWORK: &str = "artwork";
pub const CAP_PLAYLISTS: &str = "playlists";

/// Michi Control playback, queue, eventing, receiver, and multiroom capabilities.
pub const CAP_PLAYBACK: &str = "playback";
pub const CAP_QUEUE: &str = "queue";
pub const CAP_EVENTS: &str = "events";
pub const CAP_RECEIVERS: &str = "receivers";
pub const CAP_MULTIROOM: &str = "multiroom";

/// AirCat v1 original-file transfer & pinned Home TLS capabilities.
///
/// All five flags MUST only be advertised as `true` when the HTTPS/TLS listener
/// with RFC 7469 SPKI pinning is configured AND actively serving.
pub const CAP_AIRCAT_ORIGINAL_V1: &str = "aircat_original_v1";
pub const CAP_HOME_TLS_SPKI_PINNED: &str = "home_tls_spki_pinned";
pub const CAP_AIRCAT_MANIFEST_V1: &str = "aircat_manifest_v1";
pub const CAP_AIRCAT_AUTHORIZE_V1: &str = "aircat_authorize_v1";
pub const CAP_AIRCAT_ORIGINAL_DOWNLOAD_V1: &str = "aircat_original_download_v1";

/// Normative list of all five AirCat original-file capability flags.
pub const AIRCAT_REQUIRED_CAPABILITIES: &[&str] = &[
    CAP_AIRCAT_ORIGINAL_V1,
    CAP_HOME_TLS_SPKI_PINNED,
    CAP_AIRCAT_MANIFEST_V1,
    CAP_AIRCAT_AUTHORIZE_V1,
    CAP_AIRCAT_ORIGINAL_DOWNLOAD_V1,
];

/// Complete canonical server capability flag registry.
pub const CANONICAL_SERVER_CAPABILITIES: &[&str] = &[
    CAP_LIBRARY,
    CAP_SEARCH,
    CAP_STREAMING,
    CAP_DOWNLOAD,
    CAP_SYNC,
    CAP_SYNC_MANIFEST,
    CAP_ARTWORK,
    CAP_PLAYLISTS,
    CAP_PLAYBACK,
    CAP_QUEUE,
    CAP_EVENTS,
    CAP_RECEIVERS,
    CAP_MULTIROOM,
    CAP_AIRCAT_ORIGINAL_V1,
    CAP_HOME_TLS_SPKI_PINNED,
    CAP_AIRCAT_MANIFEST_V1,
    CAP_AIRCAT_AUTHORIZE_V1,
    CAP_AIRCAT_ORIGINAL_DOWNLOAD_V1,
];

/// Named runtime capability inputs used to construct a single authoritative
/// `CanonicalServerCapabilities` snapshot without positional-argument ambiguity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCapabilityState {
    pub library: bool,
    pub search: bool,
    pub streaming: bool,
    pub download: bool,
    pub sync: bool,
    pub sync_manifest: bool,
    pub artwork: bool,
    pub playlists: bool,
    pub playback: bool,
    pub queue: bool,
    pub events: bool,
    pub receivers: bool,
    pub multiroom: bool,
    /// True ONLY when the HTTPS/TLS listener is actively bound with a valid cert/SAN/key.
    pub aircat_tls_listener_ready: bool,
    /// True ONLY when the AirCat original-file producer (e.g. library module) is active.
    pub aircat_producer_ready: bool,
}

/// Canonical capability state used to project consistent feature flags across
/// `/api/v1/server/info`, `/api/v1/capabilities`, and Whisker discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalServerCapabilities {
    pub library: bool,
    pub search: bool,
    pub streaming: bool,
    pub download: bool,
    pub sync: bool,
    pub sync_manifest: bool,
    pub artwork: bool,
    pub playlists: bool,
    pub playback: bool,
    pub queue: bool,
    pub events: bool,
    pub receivers: bool,
    pub multiroom: bool,
    pub aircat_original_v1: bool,
    pub home_tls_spki_pinned: bool,
    pub aircat_manifest_v1: bool,
    pub aircat_authorize_v1: bool,
    pub aircat_original_download_v1: bool,
}

impl CanonicalServerCapabilities {
    /// Constructs a canonical server capability snapshot from named runtime inputs.
    ///
    /// All five AirCat original-file flags are `true` ONLY when BOTH
    /// `aircat_tls_listener_ready` and `aircat_producer_ready` are `true`.
    pub fn from_runtime_state(input: &RuntimeCapabilityState) -> Self {
        let aircat_ready = input.aircat_tls_listener_ready && input.aircat_producer_ready;
        Self {
            library: input.library,
            search: input.search,
            streaming: input.streaming,
            download: input.download,
            sync: input.sync,
            sync_manifest: input.sync_manifest,
            artwork: input.artwork,
            playlists: input.playlists,
            playback: input.playback,
            queue: input.queue,
            events: input.events,
            receivers: input.receivers,
            multiroom: input.multiroom,
            aircat_original_v1: aircat_ready,
            home_tls_spki_pinned: input.aircat_tls_listener_ready,
            aircat_manifest_v1: aircat_ready,
            aircat_authorize_v1: aircat_ready,
            aircat_original_download_v1: aircat_ready,
        }
    }

    /// Backwards-compatible positional constructor delegating to `from_runtime_state`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_runtime(
        library: bool,
        search: bool,
        streaming: bool,
        download: bool,
        sync: bool,
        artwork: bool,
        playlists: bool,
        playback: bool,
        queue: bool,
        events: bool,
        receivers: bool,
        multiroom: bool,
        aircat_tls_listener_ready: bool,
    ) -> Self {
        Self::from_runtime_state(&RuntimeCapabilityState {
            library,
            search,
            streaming,
            download,
            sync,
            sync_manifest: sync,
            artwork,
            playlists,
            playback,
            queue,
            events,
            receivers,
            multiroom,
            aircat_tls_listener_ready,
            aircat_producer_ready: library && aircat_tls_listener_ready,
        })
    }

    /// Deterministic map representation for `/api/v1/server/info` (`features`)
    /// and contract validation.
    pub fn to_features_map(&self) -> BTreeMap<String, bool> {
        let mut map = BTreeMap::new();
        map.insert(CAP_LIBRARY.into(), self.library);
        map.insert(CAP_SEARCH.into(), self.search);
        map.insert(CAP_STREAMING.into(), self.streaming);
        map.insert(CAP_DOWNLOAD.into(), self.download);
        map.insert(CAP_SYNC.into(), self.sync);
        map.insert(CAP_SYNC_MANIFEST.into(), self.sync_manifest);
        map.insert(CAP_ARTWORK.into(), self.artwork);
        map.insert(CAP_PLAYLISTS.into(), self.playlists);
        map.insert(CAP_PLAYBACK.into(), self.playback);
        map.insert(CAP_QUEUE.into(), self.queue);
        map.insert(CAP_EVENTS.into(), self.events);
        map.insert(CAP_RECEIVERS.into(), self.receivers);
        map.insert(CAP_MULTIROOM.into(), self.multiroom);
        map.insert(CAP_AIRCAT_ORIGINAL_V1.into(), self.aircat_original_v1);
        map.insert(CAP_HOME_TLS_SPKI_PINNED.into(), self.home_tls_spki_pinned);
        map.insert(CAP_AIRCAT_MANIFEST_V1.into(), self.aircat_manifest_v1);
        map.insert(CAP_AIRCAT_AUTHORIZE_V1.into(), self.aircat_authorize_v1);
        map.insert(
            CAP_AIRCAT_ORIGINAL_DOWNLOAD_V1.into(),
            self.aircat_original_download_v1,
        );
        map
    }

    /// Sorted list of enabled canonical feature names (for `/api/v1/capabilities`).
    pub fn enabled_feature_names(&self) -> Vec<String> {
        self.to_features_map()
            .into_iter()
            .filter_map(|(k, v)| if v { Some(k) } else { None })
            .collect()
    }

    /// Compact discovery feature projection for Whisker UDP multicast announcements.
    ///
    /// Includes core server capabilities plus `aircat_original_v1` and
    /// `home_tls_spki_pinned` ONLY when HTTPS/TLS is actually active.
    pub fn to_discovery_features(&self) -> BTreeMap<String, bool> {
        let mut map = BTreeMap::new();
        map.insert(CAP_LIBRARY.into(), self.library);
        map.insert(CAP_STREAMING.into(), self.streaming);
        map.insert(CAP_DOWNLOAD.into(), self.download);
        map.insert(CAP_PLAYBACK.into(), self.playback);
        map.insert(CAP_QUEUE.into(), self.queue);
        map.insert(CAP_SYNC.into(), self.sync);
        map.insert(CAP_AIRCAT_ORIGINAL_V1.into(), self.aircat_original_v1);
        map.insert(CAP_HOME_TLS_SPKI_PINNED.into(), self.home_tls_spki_pinned);
        map
    }
}

/// Returns `true` ONLY when all five normative AirCat original-file capability
/// flags are advertised as `true`. Advertisement alone never authorizes access.
pub fn is_aircat_original_v1_ready(features: &BTreeMap<String, bool>) -> bool {
    AIRCAT_REQUIRED_CAPABILITIES
        .iter()
        .all(|cap| features.get(*cap).copied() == Some(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aircat_flags_off_when_tls_listener_not_ready() {
        let caps = CanonicalServerCapabilities::from_runtime(
            true, true, true, true, true, true, true, true, true, true, true, true, false,
        );
        let map = caps.to_features_map();
        assert!(!is_aircat_original_v1_ready(&map));
        for key in AIRCAT_REQUIRED_CAPABILITIES {
            assert_eq!(map.get(*key), Some(&false), "expected {key} = false");
        }
        let enabled = caps.enabled_feature_names();
        for key in AIRCAT_REQUIRED_CAPABILITIES {
            assert!(
                !enabled.iter().any(|k| k == key),
                "{key} must not appear in enabled list without TLS"
            );
        }
        assert_eq!(
            caps.to_discovery_features().get(CAP_AIRCAT_ORIGINAL_V1),
            Some(&false)
        );
    }

    #[test]
    fn test_aircat_flags_all_on_when_tls_listener_ready() {
        let caps = CanonicalServerCapabilities::from_runtime(
            true, true, true, true, true, true, true, true, true, true, true, true, true,
        );
        let map = caps.to_features_map();
        assert!(is_aircat_original_v1_ready(&map));
        for key in AIRCAT_REQUIRED_CAPABILITIES {
            assert_eq!(map.get(*key), Some(&true), "expected {key} = true");
        }
        let enabled = caps.enabled_feature_names();
        for key in AIRCAT_REQUIRED_CAPABILITIES {
            assert!(enabled.iter().any(|k| k == key));
        }
        let disc = caps.to_discovery_features();
        assert_eq!(disc.get(CAP_AIRCAT_ORIGINAL_V1), Some(&true));
        assert_eq!(disc.get(CAP_HOME_TLS_SPKI_PINNED), Some(&true));
    }

    #[test]
    fn test_partial_aircat_advertisement_is_not_ready() {
        let mut partial = BTreeMap::new();
        partial.insert(CAP_AIRCAT_ORIGINAL_V1.into(), true);
        partial.insert(CAP_HOME_TLS_SPKI_PINNED.into(), true);
        assert!(!is_aircat_original_v1_ready(&partial));
    }

    #[test]
    fn test_from_runtime_state_requires_both_tls_and_producer_for_aircat() {
        let caps = CanonicalServerCapabilities::from_runtime_state(&RuntimeCapabilityState {
            library: false,
            search: true,
            streaming: true,
            download: true,
            sync: true,
            sync_manifest: false,
            artwork: true,
            playlists: true,
            playback: true,
            queue: true,
            events: true,
            receivers: true,
            multiroom: false,
            aircat_tls_listener_ready: true,
            aircat_producer_ready: false,
        });
        assert!(caps.home_tls_spki_pinned);
        assert!(!caps.aircat_original_v1);
        assert!(!caps.aircat_manifest_v1);
        assert!(!caps.aircat_authorize_v1);
        assert!(!caps.aircat_original_download_v1);
        assert!(!is_aircat_original_v1_ready(&caps.to_features_map()));
        assert!(!caps.sync_manifest);
        assert!(!caps.multiroom);
    }
}
