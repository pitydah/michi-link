//! Discovery demonstration with signed announces.
//!
//! Shows:
//! 1. Alice and Bob identities.
//! 2. Alice builds a fully signed announce with a STABLE device_id.
//! 3. Bob verifies it (signature, michi_id derivation, timestamp window).
//! 4. Legacy (unsigned) announces are classified as untrusted.
//! 5. Replay of the same announce is rejected.
use std::collections::BTreeMap;
use std::sync::Arc;
use tempfile::TempDir;

fn main() {
    let dir = TempDir::new().expect("failed to create temp dir");

    println!("=== Michi Discovery Demo ===\n");
    println!(
        "Canonical multicast: {}:{}",
        michi_identity::discovery::MULTICAST_GROUP,
        michi_identity::discovery::MULTICAST_PORT
    );

    // 1. Create identities
    println!("\n1. Creating identities...");
    let alice = Arc::new(
        michi_identity::IdentityManager::generate(dir.path(), "Alice", "pw")
            .expect("alice generation failed"),
    );
    let bob = Arc::new(
        michi_identity::IdentityManager::generate(dir.path(), "Bob", "pw")
            .expect("bob generation failed"),
    );

    println!("   Alice Michi ID: {}", alice.michi_id());
    println!("   Bob Michi ID:   {}", bob.michi_id());

    // 2. Alice builds a signed announce with a stable device_id
    println!("\n2. Alice builds signed announce...");
    let alice_engine = michi_identity::DiscoveryEngine::new(alice.clone());
    let mut features = BTreeMap::new();
    features.insert("library".to_string(), true);
    features.insert("events".to_string(), false);

    let profile = michi_identity::AnnounceProfile {
        device_id: "alice-player-01".into(),
        name: alice.device_name().into(),
        service: michi_identity::Service::MusicPlayer,
        api_version: michi_identity::ApiVersion::V1,
        roles: vec![
            michi_identity::Role::DesktopPlayer,
            michi_identity::Role::LibraryMaster,
        ],
        host: "192.168.1.10".into(),
        port: 8400,
        features,
    };
    let announce = alice_engine
        .build_signed_announce(&profile)
        .expect("profile validation failed");
    println!("   Device id:     {}", announce.device_id);
    println!("   Service:       {}", announce.service);
    println!("   Has signature: {}", announce.signature.is_some());
    println!(
        "   Canonical payload: {} bytes",
        michi_identity::DiscoveryEngine::canonical_bytes(&announce).len()
    );

    // 3. Bob verifies Alice's announce
    println!("\n3. Bob verifies Alice's announce...");
    let bob_engine = michi_identity::DiscoveryEngine::new(bob.clone());
    let trust = bob_engine
        .verify_announce(&announce, None)
        .expect("verify failed");

    match trust {
        michi_identity::TrustLevel::Verified(michi_id) => {
            println!("   ✅ Verified! Alice's Michi ID: {}", michi_id);
        }
        michi_identity::TrustLevel::Untrusted(id) => {
            println!("   ⚠️  Untrusted (legacy): {}", id);
        }
        michi_identity::TrustLevel::Invalid => {
            println!("   ❌ Invalid signature!");
        }
    }

    // 4. Legacy announce (unsigned) is untrusted
    println!("\n4. Bob verifies legacy announce (no signature)...");
    let legacy_announce = michi_identity::Announce {
        device_id: "legacy-device-01".into(),
        name: "Legacy Device".into(),
        service: michi_identity::Service::MusicPlayer,
        roles: vec![michi_identity::Role::DesktopPlayer],
        api_version: michi_identity::ApiVersion::V1,
        host: "192.168.1.100".into(),
        port: 8400,
        features: BTreeMap::new(),
        michi_id: None,
        public_key: None,
        signature: None,
        timestamp_ms: None,
        nonce: None,
    };

    let trust = bob_engine
        .verify_announce(&legacy_announce, None)
        .expect("verify failed");
    match trust {
        michi_identity::TrustLevel::Untrusted(_) => {
            println!("   ✅ Accepted as untrusted (legacy compatible)");
        }
        _ => println!("   ❌ Unexpected trust level"),
    }

    // 5. Replay is rejected
    println!("\n5. Replaying the same announce...");
    match bob_engine.verify_announce(&announce, None) {
        Err(e) => println!("   ✅ Replay correctly rejected: {}", e),
        Ok(_) => println!("   ❌ Replay accepted (should not happen)"),
    }

    println!("\n=== Demo complete ===");
}
