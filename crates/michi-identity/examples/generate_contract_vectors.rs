//! Contract golden vectors generator.
//!
//! Emits real JSON payloads produced by the Rust implementation so the
//! cross-layer suite (`tests/cross_layer/validate-generated.js`) can validate
//! them against the JSON Schemas with AJV.
//!
//! ## Determinism
//!
//! The generator is deterministic by construction: Ed25519 identities are
//! derived from fixed seeds (never `OsRng`) and every timestamp, nonce, UUID
//! and PIN is frozen, so two runs produce byte-identical output. This is what
//! makes the committed vectors and the receiver v1-lite conformance bundle
//! reproducible (`BUNDLE_PASS`).
//!
//! ## Usage
//!
//! ```text
//! cargo run --example generate_contract_vectors -- <output-dir> [--receiver <dir>]
//! ```
//!
//! - `<output-dir>`: canonical golden vectors (default:
//!   `tests/vectors/generated` relative to the repository root, or the
//!   current directory when run from the crate).
//! - `--receiver <dir>`: additionally emits the receiver v1-lite conformance
//!   vectors consumed by `scripts/build-receiver-bundle.py` (identity,
//!   discovery, pairing, positive and negative examples).
//!
//! Generated files must not be edited by hand: regenerate instead.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;

use michi_identity::{
    decode_base64url_strict, encode_base64url, Announce, AnnounceProfile, ApiVersion, AuthStrategy,
    DiscoveryEngine, IdentityDocument, IdentityManager, MichiId, PairConfirmRequest,
    PairConfirmResponse, PairStartRequest, PairStartResponse, QRConnector, Role, Service,
};

/// Domain separator for deterministic test-vector seeds.
const SEED_CONTEXT: &[u8] = b"michi-link contract vectors v1";

/// Frozen wall-clock values (never read from the clock in this generator).
const FROZEN_CREATED_AT: &str = "2026-01-01T00:00:00Z";
const FROZEN_PAIR_EXPIRES_AT: &str = "2026-01-01T00:05:00Z";
const FROZEN_TIMESTAMP_MS: i64 = 1_767_225_600_000; // 2026-01-01T00:00:00Z

/// Frozen UUIDs shared across pairing/session vectors.
const PAIR_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440001";
const STREAM_SERVER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const CONTROLLER_DEVICE_ID: &str = "550e8400-e29b-41d4-a716-446655440002";
const STREAM_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440003";

/// Frozen receiver PIN displayed during pairing.
const RECEIVER_PIN: &str = "042731";

/// Derives a deterministic 32-byte seed for a named vector identity.
fn derive_seed(label: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(SEED_CONTEXT);
    hasher.update(label.as_bytes());
    *hasher.finalize().as_bytes()
}

/// Deterministic Ed25519 identity derived from a fixed seed.
struct DeterministicIdentity {
    signing_key: SigningKey,
    michi_id: MichiId,
    public_key: String,
}

impl DeterministicIdentity {
    fn from_seed(seed: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            michi_id: MichiId::from_public_key(&verifying_key),
            public_key: encode_base64url(&verifying_key.to_bytes()),
        }
    }

    /// Signs `payload` and returns the signature in canonical base64url.
    fn sign(&self, payload: &[u8]) -> String {
        encode_base64url(&self.signing_key.sign(payload).to_bytes())
    }

    /// Signs an announce over its canonical payload.
    fn sign_announce(&self, announce: &Announce) -> String {
        self.sign(&DiscoveryEngine::canonical_bytes(announce))
    }
}

/// Fixed nonce derived deterministically from a tag byte.
fn fixed_nonce(tag: u8) -> [u8; 16] {
    let mut nonce = [0u8; 16];
    nonce[0] = tag;
    for (index, byte) in nonce.iter_mut().enumerate().skip(1) {
        *byte = index as u8 * 7 + tag;
    }
    nonce
}

fn write_json(dir: &Path, name: &str, value: &impl Serialize) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let json = serde_json::to_string_pretty(value).unwrap();
    std::fs::write(&path, json + "\n").unwrap();
    println!("wrote {}", path.display());
}

/// Self-check: the emitted announce signature must verify and the declared
/// `michi_id` must derive from the announced public key.
fn assert_announce_valid(announce: &Announce) {
    let canonical = DiscoveryEngine::canonical_bytes(announce);
    let signature = announce.signature.as_deref().unwrap();
    let public_key = announce.public_key.as_deref().unwrap();
    assert!(
        IdentityManager::verify(&canonical, signature, public_key).unwrap(),
        "announce signature must verify"
    );
    let derived = IdentityManager::derive_michi_id(public_key).unwrap();
    assert_eq!(
        derived.to_base64url(),
        announce.michi_id.as_deref().unwrap(),
        "announced michi_id must derive from the announced public key"
    );
}

/// Builds a fully signed announce for a deterministic identity.
fn signed_announce(
    identity: &DeterministicIdentity,
    profile: &AnnounceProfile,
    timestamp_ms: i64,
    nonce: [u8; 16],
) -> Announce {
    let mut announce = Announce {
        device_id: profile.device_id.clone(),
        name: profile.name.clone(),
        service: profile.service,
        roles: profile.roles.clone(),
        api_version: profile.api_version,
        host: profile.host.clone(),
        port: profile.port,
        features: profile.features.clone(),
        michi_id: Some(identity.michi_id.to_base64url()),
        public_key: Some(identity.public_key.clone()),
        signature: None,
        timestamp_ms: Some(timestamp_ms),
        nonce: Some(encode_base64url(&nonce)),
    };
    announce.signature = Some(identity.sign_announce(&announce));
    assert_announce_valid(&announce);
    announce
}

fn main() {
    let mut out_dir: Option<PathBuf> = None;
    let mut receiver_dir: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--receiver" => {
                let dir = args
                    .next()
                    .expect("--receiver requires a directory argument");
                receiver_dir = Some(PathBuf::from(dir));
            }
            _ => {
                assert!(
                    !arg.starts_with('-'),
                    "unknown option: {} (usage: generate_contract_vectors <out-dir> [--receiver <dir>])",
                    arg
                );
                out_dir = Some(PathBuf::from(arg));
            }
        }
    }
    let out_dir = out_dir.unwrap_or_else(|| {
        // Default: repository-root/tests/vectors/generated when invoked
        // from the crate directory via cargo.
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir.join("../../tests/vectors/generated")
    });
    std::fs::create_dir_all(&out_dir).unwrap();

    let device = DeterministicIdentity::from_seed(derive_seed("device"));
    let server = DeterministicIdentity::from_seed(derive_seed("server"));
    let client = DeterministicIdentity::from_seed(derive_seed("client"));

    // --- Identity document ---
    let identity_document = IdentityDocument {
        michi_id: device.michi_id,
        public_key: device.public_key.clone(),
        identity_scheme: "ed25519-blake3-v1".into(),
        device_name: "Vector Device".into(),
        created_at: FROZEN_CREATED_AT.into(),
    };
    write_json(
        &out_dir,
        "identity-document.generated.json",
        &identity_document,
    );

    // --- Signed announces for the five active services ---
    let player_profile = AnnounceProfile {
        device_id: "vector-player-01".into(),
        name: "Michi Music Player".into(),
        service: Service::MusicPlayer,
        api_version: ApiVersion::V1,
        roles: vec![Role::DesktopPlayer, Role::LibraryMaster, Role::SyncHost],
        host: "192.168.1.100".into(),
        port: 8400,
        features: BTreeMap::from([
            ("library".to_string(), true),
            ("streaming".to_string(), true),
            ("remote_control".to_string(), true),
            ("events".to_string(), false),
        ]),
    };
    assert!(DiscoveryEngine::validate_profile(&player_profile).is_ok());
    write_json(
        &out_dir,
        "announce-player.generated.json",
        &signed_announce(
            &device,
            &player_profile,
            FROZEN_TIMESTAMP_MS,
            fixed_nonce(1),
        ),
    );

    let micro_profile = AnnounceProfile {
        device_id: "vector-micro-01".into(),
        name: "Michi Micro Server".into(),
        service: Service::MicroServer,
        api_version: ApiVersion::V1,
        roles: vec![Role::MusicServer, Role::LibraryHost, Role::PlaybackHost],
        host: "192.168.1.101".into(),
        port: 8500,
        features: BTreeMap::from([
            ("library".to_string(), true),
            ("streaming".to_string(), true),
            ("downloads".to_string(), true),
            ("sync".to_string(), true),
            ("playback".to_string(), true),
            ("queue".to_string(), true),
            ("import".to_string(), true),
            ("events".to_string(), false),
        ]),
    };
    assert!(DiscoveryEngine::validate_profile(&micro_profile).is_ok());
    write_json(
        &out_dir,
        "announce-micro.generated.json",
        &signed_announce(
            &device,
            &micro_profile,
            FROZEN_TIMESTAMP_MS + 1_000,
            fixed_nonce(2),
        ),
    );

    let stream_profile = AnnounceProfile {
        device_id: "vector-stream-01".into(),
        name: "Michi Stream Hi-Fi".into(),
        service: Service::StreamHiFi,
        api_version: ApiVersion::V1Lite,
        roles: vec![Role::AudioReceiver],
        host: "192.168.1.102".into(),
        port: 8600,
        features: BTreeMap::from([
            ("session".to_string(), true),
            ("volume".to_string(), true),
            ("heartbeat".to_string(), true),
        ]),
    };
    assert!(DiscoveryEngine::validate_profile(&stream_profile).is_ok());
    write_json(
        &out_dir,
        "announce-stream.generated.json",
        &signed_announce(
            &device,
            &stream_profile,
            FROZEN_TIMESTAMP_MS + 2_000,
            fixed_nonce(3),
        ),
    );

    // --- Pairing DTOs ---
    let nonce = fixed_nonce(4);
    let challenge_signature = client.sign(&nonce);
    assert!(
        IdentityManager::verify(&nonce, &challenge_signature, &client.public_key).unwrap(),
        "challenge signature must verify"
    );
    let pair_start_request = PairStartRequest {
        device_name: "Michi Mobile".into(),
        device_type: "mobile".into(),
        roles: vec![Role::MobilePlayer, Role::RemoteController, Role::SyncClient],
        auth_strategy: AuthStrategy::Ed25519Challenge,
        michi_id: client.michi_id.to_base64url(),
        public_key: client.public_key.clone(),
        challenge_nonce: encode_base64url(&nonce),
        challenge_signature,
    };
    write_json(
        &out_dir,
        "pair-start-request.generated.json",
        &pair_start_request,
    );

    let pair_start_response = PairStartResponse {
        session_id: uuid::Uuid::parse_str(PAIR_SESSION_ID).unwrap(),
        expires_at: FROZEN_PAIR_EXPIRES_AT.into(),
        attempts_remaining: 5,
        server_michi_id: server.michi_id.to_base64url(),
        server_public_key: server.public_key.clone(),
    };
    write_json(
        &out_dir,
        "pair-start-response.generated.json",
        &pair_start_response,
    );

    let pair_confirm_request = PairConfirmRequest {
        session_id: uuid::Uuid::parse_str(PAIR_SESSION_ID).unwrap(),
        pin: RECEIVER_PIN.into(),
        michi_id: client.michi_id.to_base64url(),
        public_key: client.public_key.clone(),
    };
    write_json(
        &out_dir,
        "pair-confirm-request.generated.json",
        &pair_confirm_request,
    );

    // The token is minted by the server HTTP layer; the DTO documents the shape.
    let pair_confirm_response = PairConfirmResponse {
        token: "tok_opaque_vector_example".into(),
        refresh_token: Some("tok_opaque_refresh_vector_example".into()),
        expires_in: 3600,
        device_id: client.michi_id.to_base64url(),
        server_id: server.michi_id.to_base64url(),
    };
    write_json(
        &out_dir,
        "pair-confirm-response.generated.json",
        &pair_confirm_response,
    );

    // --- QR payload ---
    let pairing_session_id = uuid::Uuid::parse_str(PAIR_SESSION_ID).unwrap();
    let endpoint = url::Url::parse("http://192.168.1.50:8400").unwrap();
    let uri = QRConnector::build_uri(
        &server.michi_id.to_base64url(),
        &server.public_key,
        pairing_session_id,
        FROZEN_PAIR_EXPIRES_AT,
        &endpoint,
    )
    .unwrap();
    // Emit both the URI and its parsed payload for cross-checks.
    let parsed = QRConnector::parse_pairing_uri(&uri).unwrap();
    write_json(
        &out_dir,
        "qr-payload.generated.json",
        &serde_json::json!({
            "uri": uri,
            "format": parsed.format,
            "version": parsed.version,
            "server_michi_id": parsed.server_michi_id,
            "server_public_key": parsed.server_public_key,
            "session_id": parsed.session_id.to_string(),
            "expires_at": parsed.expires_at,
            "endpoint": parsed.endpoint.as_str(),
        }),
    );

    if let Some(receiver_dir) = receiver_dir {
        emit_receiver_vectors(&receiver_dir);
    }

    println!("golden vectors written to {}", out_dir.display());
}

/// Emits the receiver v1-lite conformance vectors consumed by
/// `scripts/build-receiver-bundle.py`. Everything is deterministic.
fn emit_receiver_vectors(dir: &Path) {
    let receiver = DeterministicIdentity::from_seed(derive_seed("receiver"));
    let micro = DeterministicIdentity::from_seed(derive_seed("micro-server"));

    // --- Identity: canonical server/info payloads (Standard and Hi-Fi) ---
    let server_info = |service: &str| {
        serde_json::json!({
            "service": service,
            "name": "Michi Stream Cocina",
            "server_id": STREAM_SERVER_ID,
            "version": "0.3.0",
            "api_version": "v1-lite",
            "roles": ["audio_receiver"],
            "identity_scheme": "ed25519-blake3-v1",
            "michi_id": receiver.michi_id.to_base64url(),
            "public_key": receiver.public_key,
            "auth": {
                "required": true,
                "strategy": "RECEIVER_BUTTON",
                "token_refresh": false
            },
            "features": {
                "session": true,
                "heartbeat": true,
                "volume": true,
                "now_playing": true,
                "diagnostics": true,
                "ota": true
            },
            "audio": {
                "transports": ["rtp_udp"],
                "codecs": ["pcm_s16le"],
                "sample_rates": [48000],
                "bit_depths": [16],
                "channels": [2],
                "packet_ms": [10],
                "payload_types": [97],
                "buffer_ms_min": 50,
                "buffer_ms_max": 500
            }
        })
    };
    write_json(
        dir,
        "identity/server-info-standard.json",
        &server_info("michi-stream-standard"),
    );
    write_json(
        dir,
        "identity/server-info-hifi.json",
        &server_info("michi-stream-hifi"),
    );

    // --- Discovery: valid signed announce + announce with altered signature ---
    let announce_profile = AnnounceProfile {
        device_id: STREAM_SERVER_ID.into(),
        name: "Michi Stream Cocina".into(),
        service: Service::StreamStandard,
        api_version: ApiVersion::V1Lite,
        roles: vec![Role::AudioReceiver],
        host: "192.168.1.102".into(),
        port: 8600,
        features: BTreeMap::from([
            ("session".to_string(), true),
            ("volume".to_string(), true),
            ("heartbeat".to_string(), true),
        ]),
    };
    assert!(DiscoveryEngine::validate_profile(&announce_profile).is_ok());
    let announce = signed_announce(
        &receiver,
        &announce_profile,
        FROZEN_TIMESTAMP_MS,
        fixed_nonce(10),
    );
    write_json(dir, "discovery/announce-valid.json", &announce);

    let mut signature_altered = announce.clone();
    let mut signature_bytes =
        decode_base64url_strict(signature_altered.signature.as_deref().unwrap())
            .expect("signature must decode");
    signature_bytes[0] ^= 0x01;
    signature_altered.signature = Some(encode_base64url(&signature_bytes));
    let canonical = DiscoveryEngine::canonical_bytes(&signature_altered);
    assert!(
        !IdentityManager::verify(
            &canonical,
            signature_altered.signature.as_deref().unwrap(),
            signature_altered.public_key.as_deref().unwrap(),
        )
        .unwrap(),
        "the altered signature must NOT verify"
    );
    write_json(
        dir,
        "discovery/announce-signature-altered.json",
        &signature_altered,
    );

    // --- Pairing: RECEIVER_BUTTON flow initiated by a `server` device ---
    let challenge_nonce = fixed_nonce(11);
    let challenge_signature = micro.sign(&challenge_nonce);
    assert!(
        IdentityManager::verify(&challenge_nonce, &challenge_signature, &micro.public_key).unwrap(),
        "challenge signature must verify"
    );
    let pair_start = PairStartRequest {
        device_name: "Michi Micro Server".into(),
        device_type: "server".into(),
        roles: vec![Role::MusicServer],
        auth_strategy: AuthStrategy::ReceiverButton,
        michi_id: micro.michi_id.to_base64url(),
        public_key: micro.public_key.clone(),
        challenge_nonce: encode_base64url(&challenge_nonce),
        challenge_signature,
    };
    write_json(dir, "pairing/pair-start-valid.json", &pair_start);

    // Altered nonce: the signature no longer matches the nonce bytes.
    let altered_nonce = fixed_nonce(12);
    let pair_start_nonce_altered = PairStartRequest {
        challenge_nonce: encode_base64url(&altered_nonce),
        ..pair_start.clone()
    };
    assert!(
        !IdentityManager::verify(
            &altered_nonce,
            &pair_start_nonce_altered.challenge_signature,
            &pair_start_nonce_altered.public_key,
        )
        .unwrap(),
        "the signature over the altered nonce must NOT verify"
    );
    write_json(
        dir,
        "pairing/pair-start-nonce-altered.json",
        &pair_start_nonce_altered,
    );

    // Incoherent michi_id: does not derive from the announced public key.
    let pair_start_wrong_id = PairStartRequest {
        michi_id: receiver.michi_id.to_base64url(),
        ..pair_start.clone()
    };
    let derived = IdentityManager::derive_michi_id(&pair_start_wrong_id.public_key).unwrap();
    assert_ne!(
        derived.to_base64url(),
        pair_start_wrong_id.michi_id,
        "the substituted michi_id must not derive from the announced public key"
    );
    write_json(
        dir,
        "pairing/pair-start-wrong-michi-id.json",
        &pair_start_wrong_id,
    );

    let pair_confirm = PairConfirmRequest {
        session_id: uuid::Uuid::parse_str(PAIR_SESSION_ID).unwrap(),
        pin: RECEIVER_PIN.into(),
        michi_id: micro.michi_id.to_base64url(),
        public_key: micro.public_key.clone(),
    };
    write_json(dir, "pairing/pair-confirm-valid.json", &pair_confirm);
    write_json(
        dir,
        "pairing/pair-confirm-wrong-pin.json",
        &PairConfirmRequest {
            pin: "000000".into(),
            ..pair_confirm.clone()
        },
    );
    // A replay is the exact same request sent again.
    write_json(dir, "pairing/pair-confirm-replay.json", &pair_confirm);

    // Receiver-issued token: no automatic expiry, no refresh token.
    let pair_confirm_response = PairConfirmResponse {
        token: "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE".into(),
        refresh_token: None,
        expires_in: 0,
        device_id: CONTROLLER_DEVICE_ID.into(),
        server_id: STREAM_SERVER_ID.into(),
    };
    write_json(
        dir,
        "pairing/pair-confirm-response.json",
        &pair_confirm_response,
    );

    // --- Positive examples: canonical session and heartbeat payloads ---
    write_json(
        dir,
        "examples/positive/receiver-session-create.json",
        &serde_json::json!({
            "transport": "rtp_udp",
            "codec": "pcm_s16le",
            "sample_rate": 48000,
            "bit_depth": 16,
            "channels": 2,
            "packet_ms": 10,
            "buffer_ms": 120,
            "payload_type": 97,
            "ssrc": 305419896,
            "volume": 70
        }),
    );
    write_json(
        dir,
        "examples/positive/receiver-session-created.json",
        &serde_json::json!({
            "session_id": STREAM_SESSION_ID,
            "session_token": "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
            "lease_seconds": 30,
            "effective": {
                "transport": "rtp_udp",
                "codec": "pcm_s16le",
                "sample_rate": 48000,
                "bit_depth": 16,
                "channels": 2,
                "packet_ms": 10,
                "buffer_ms": 120,
                "payload_type": 97,
                "ssrc": 305419896,
                "stream_port": 55300,
                "volume": 70
            }
        }),
    );
    write_json(
        dir,
        "examples/positive/receiver-session-status.json",
        &serde_json::json!({
            "session_id": STREAM_SESSION_ID,
            "state": "playing",
            "lease_remaining_ms": 24500,
            "volume": 70,
            "paused": false,
            "stream_port": 55300,
            "ssrc": 305419896,
            "packets_received": 1250,
            "packets_rejected": 0,
            "packets_lost": 0,
            "underruns": 0
        }),
    );
    write_json(
        dir,
        "examples/positive/receiver-session-patch.json",
        &serde_json::json!({
            "volume": 55,
            "paused": true
        }),
    );
    write_json(
        dir,
        "examples/positive/receiver-heartbeat.json",
        &serde_json::json!({
            "session_id": STREAM_SESSION_ID,
            "sequence": 7,
            "sent_at_ms": 1786564800000u64
        }),
    );
    write_json(
        dir,
        "examples/positive/receiver-heartbeat-response.json",
        &serde_json::json!({
            "session_id": STREAM_SESSION_ID,
            "status": "alive",
            "lease_seconds": 30,
            "receiver_uptime_ms": 918273
        }),
    );

    // --- Negative examples: replay/stale heartbeats and the canonical
    // error envelope, one per code of the §2.7 map. ---
    write_json(
        dir,
        "examples/negative/receiver-heartbeat-repeated.json",
        &serde_json::json!({
            "session_id": STREAM_SESSION_ID,
            "sequence": 7,
            "sent_at_ms": 1786564810000u64
        }),
    );
    write_json(
        dir,
        "examples/negative/receiver-heartbeat-stale.json",
        &serde_json::json!({
            "session_id": STREAM_SESSION_ID,
            "sequence": 3,
            "sent_at_ms": 1786564770000u64
        }),
    );

    let error_body =
        |code: &str, message: &str, request_id: &str, details: Option<serde_json::Value>| {
            let mut body = serde_json::json!({
                "error": {
                    "code": code,
                    "message": message,
                    "request_id": request_id
                }
            });
            if let Some(details) = details {
                body["error"]["details"] = details;
            }
            body
        };
    write_json(
        dir,
        "examples/negative/error-invalid-request.json",
        &error_body(
            "INVALID_REQUEST",
            "buffer_ms must be between 50 and 500",
            "550e8400-e29b-41d4-a716-446655440100",
            Some(serde_json::json!({"field": "buffer_ms"})),
        ),
    );
    write_json(
        dir,
        "examples/negative/error-unauthorized.json",
        &error_body(
            "UNAUTHORIZED",
            "missing or invalid bearer token",
            "550e8400-e29b-41d4-a716-446655440101",
            None,
        ),
    );
    write_json(
        dir,
        "examples/negative/error-forbidden.json",
        &error_body(
            "FORBIDDEN",
            "pairing window is closed",
            "550e8400-e29b-41d4-a716-446655440102",
            None,
        ),
    );
    write_json(
        dir,
        "examples/negative/error-not-found.json",
        &error_body(
            "NOT_FOUND",
            "session not found",
            "550e8400-e29b-41d4-a716-446655440103",
            None,
        ),
    );
    write_json(
        dir,
        "examples/negative/error-conflict.json",
        &error_body(
            "CONFLICT",
            "heartbeat sequence already seen",
            "550e8400-e29b-41d4-a716-446655440104",
            None,
        ),
    );
    write_json(
        dir,
        "examples/negative/error-rate-limited.json",
        &error_body(
            "RATE_LIMITED",
            "too many pairing attempts",
            "550e8400-e29b-41d4-a716-446655440105",
            None,
        ),
    );
    write_json(
        dir,
        "examples/negative/error-not-implemented.json",
        &error_body(
            "NOT_IMPLEMENTED",
            "feature not implemented",
            "550e8400-e29b-41d4-a716-446655440106",
            None,
        ),
    );
    write_json(
        dir,
        "examples/negative/error-internal.json",
        &error_body(
            "INTERNAL_ERROR",
            "unexpected internal error",
            "550e8400-e29b-41d4-a716-446655440107",
            None,
        ),
    );

    println!("receiver vectors written to {}", dir.display());
}
