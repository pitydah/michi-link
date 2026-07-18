/// Demostración de discovery con announces firmados.
///
/// Muestra:
/// 1. Dos identidades: Alice y Bob.
/// 2. Alice construye un announce firmado.
/// 3. Bob verifica el announce de Alice.
/// 4. También acepta announces legacy (sin firma).
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    let dir = TempDir::new().expect("failed to create temp dir");

    println!("=== Michi Discovery Demo ===\n");

    // 1. Crear identidades
    println!("1. Creating identities...");
    let alice = Arc::new(
        michi_identity::IdentityManager::generate(dir.path(), "Alice")
            .await
            .expect("alice generation failed"),
    );
    let bob = Arc::new(
        michi_identity::IdentityManager::generate(dir.path(), "Bob")
            .await
            .expect("bob generation failed"),
    );

    println!("   Alice Michi ID: {}", alice.michi_id());
    println!("   Bob Michi ID:   {}", bob.michi_id());

    // 2. Alice construye un announce firmado
    println!("\n2. Alice builds signed announce...");
    let alice_engine = michi_identity::DiscoveryEngine::new(alice.clone());
    let announce = alice_engine.build_signed_announce();
    println!("   Announce device: {}", announce.device_name);
    println!("   Has signature:   {}", announce.signature.is_some());

    // 3. Bob verifica el announce de Alice
    println!("\n3. Bob verifies Alice's announce...");
    let bob_engine = michi_identity::DiscoveryEngine::new(bob.clone());
    let trust = bob_engine.verify_announce(&announce).expect("verify failed");

    match trust {
        michi_identity::TrustLevel::Verified(michi_id) => {
            println!("   ✅ Verified! Alice's Michi ID: {}", michi_id);
        }
        michi_identity::TrustLevel::Untrusted(uuid) => {
            println!("   ⚠️  Untrusted (legacy): {}", uuid);
        }
        michi_identity::TrustLevel::Invalid => {
            println!("   ❌ Invalid signature!");
        }
    }

    // 4. Bob verifica un announce legacy (sin firma)
    println!("\n4. Bob verifies legacy announce (no signature)...");
    let legacy_announce = michi_identity::SignedAnnounce {
        device_id: uuid::Uuid::new_v4(),
        device_name: "Legacy Device".into(),
        device_type: "michi_server".into(),
        roles: vec!["server".into()],
        api_version: "1.0.0".into(),
        host: "192.168.1.100".into(),
        port: 8500,
        michi_id: None,
        public_key: None,
        signature: None,
        capabilities: None,
    };

    let trust = bob_engine
        .verify_announce(&legacy_announce)
        .expect("verify failed");
    match trust {
        michi_identity::TrustLevel::Untrusted(_) => {
            println!("   ✅ Accepted as untrusted (legacy compatible)");
        }
        _ => println!("   ❌ Unexpected trust level"),
    }

    println!("\n=== Demo complete ===");
}
