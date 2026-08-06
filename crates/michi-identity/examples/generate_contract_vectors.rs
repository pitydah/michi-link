//! Contract golden vectors generator.
//!
//! Emits real JSON payloads produced by the Rust implementation so the
//! cross-layer suite (`tests/cross_layer/validate-generated.js`) can validate
//! them against the JSON Schemas with AJV.
//!
//! Usage: `cargo run --example generate_contract_vectors -- <output-dir>`
//! (default: `tests/vectors/generated` relative to the repository root, or the
//! current directory when run from the crate).
//!
//! Generated files must not be edited by hand: regenerate instead.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use michi_identity::{
    encode_base64url, AnnounceProfile, ApiVersion, DiscoveryEngine, IdentityManager,
    PairConfirmRequest, PairConfirmResponse, PairStartRequest, PairingRegistry, QRConnector, Role,
    Service,
};

const PASSWORD: &str = "contract-vector-password";

fn write_json(dir: &std::path::Path, name: &str, value: &impl serde::Serialize) {
    let path = dir.join(name);
    let json = serde_json::to_string_pretty(value).unwrap();
    std::fs::write(&path, json + "\n").unwrap();
    println!("wrote {}", path.display());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            // Default: repository-root/tests/vectors/generated when invoked
            // from the crate directory via cargo.
            let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            crate_dir.join("../../tests/vectors/generated")
        }
    };
    std::fs::create_dir_all(&out_dir).unwrap();

    // --- Identity document ---
    let dir = std::env::temp_dir().join(format!("michi-vectors-{}", std::process::id()));
    let identity =
        Arc::new(IdentityManager::load_or_generate(&dir, "Vector Device", PASSWORD).unwrap());
    write_json(
        &out_dir,
        "identity-document.generated.json",
        &identity.identity_document(),
    );

    // --- Signed announces for the five active services ---
    let engine = DiscoveryEngine::new(identity.clone());
    let mut player_features = BTreeMap::new();
    player_features.insert("library".to_string(), true);
    player_features.insert("streaming".to_string(), true);
    player_features.insert("remote_control".to_string(), true);
    player_features.insert("events".to_string(), false);

    let player = AnnounceProfile {
        device_id: "vector-player-01".into(),
        name: "Michi Music Player".into(),
        service: Service::MusicPlayer,
        api_version: ApiVersion::V1,
        roles: vec![Role::DesktopPlayer, Role::LibraryMaster, Role::SyncHost],
        host: "192.168.1.100".into(),
        port: 8400,
        features: player_features,
    };
    write_json(
        &out_dir,
        "announce-player.generated.json",
        &engine.build_signed_announce(&player).unwrap(),
    );

    let micro = AnnounceProfile {
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
    write_json(
        &out_dir,
        "announce-micro.generated.json",
        &engine.build_signed_announce(&micro).unwrap(),
    );

    let stream = AnnounceProfile {
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
    write_json(
        &out_dir,
        "announce-stream.generated.json",
        &engine.build_signed_announce(&stream).unwrap(),
    );

    // --- Pairing DTOs ---
    let server = Arc::new(IdentityManager::generate(&dir, "Vector Server", PASSWORD).unwrap());
    let client = IdentityManager::generate(&dir, "Vector Client", PASSWORD).unwrap();

    let nonce: [u8; 16] = rand::random();
    let (challenge_signature, _) = client.sign_base64url(&nonce);
    let pair_start_request = PairStartRequest {
        device_name: "Michi Mobile".into(),
        device_type: "mobile".into(),
        roles: vec![Role::MobilePlayer, Role::RemoteController, Role::SyncClient],
        auth_strategy: michi_identity::AuthStrategy::Ed25519Challenge,
        michi_id: client.michi_id().to_base64url(),
        public_key: client.public_key_base64url(),
        challenge_nonce: encode_base64url(&nonce),
        challenge_signature,
    };
    write_json(
        &out_dir,
        "pair-start-request.generated.json",
        &pair_start_request,
    );

    let registry = PairingRegistry::new();
    let (start_response, pin) = registry
        .start_server(&server, &pair_start_request, "192.168.1.50")
        .unwrap();
    write_json(
        &out_dir,
        "pair-start-response.generated.json",
        &start_response,
    );

    let pair_confirm_request = PairConfirmRequest {
        session_id: start_response.session_id,
        pin,
        michi_id: client.michi_id().to_base64url(),
        public_key: client.public_key_base64url(),
    };
    write_json(
        &out_dir,
        "pair-confirm-request.generated.json",
        &pair_confirm_request,
    );

    let session = registry
        .confirm(&pair_confirm_request, "192.168.1.50")
        .unwrap();
    // The token is minted by the server HTTP layer; the DTO documents the shape.
    let pair_confirm_response = PairConfirmResponse {
        token: "tok_opaque_vector_example".into(),
        refresh_token: Some("tok_opaque_refresh_vector_example".into()),
        expires_in: 3600,
        device_id: pair_start_request.michi_id.clone(),
        server_id: session.server_michi_id.clone(),
    };
    write_json(
        &out_dir,
        "pair-confirm-response.generated.json",
        &pair_confirm_response,
    );

    // --- QR payload ---
    let qr = QRConnector::new(server.clone());
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(5);
    let endpoint = url::Url::parse("http://192.168.1.50:8400").unwrap();
    let uri = qr
        .generate_pairing_uri(start_response.session_id, expires_at, &endpoint)
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

    println!("golden vectors written to {}", out_dir.display());
}
