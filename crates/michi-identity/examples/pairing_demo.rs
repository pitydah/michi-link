//! Full unified TOFU + PIN pairing demonstration (contract v1).
//!
//! Shows:
//! 1. Server and client identities.
//! 2. Client builds a signed challenge (possession of secret key).
//! 3. Server opens a session (5-minute lifetime, 5 attempts) and shows the
//!    PIN locally — the response never carries it.
//! 4. Client confirms with the PIN (constant-time verification).
//! 5. Wrong PIN is rejected; session reuse after success is rejected.
use std::sync::Arc;
use tempfile::TempDir;

fn main() {
    let dir = TempDir::new().expect("failed to create temp dir");

    println!("=== Michi Pairing TOFU + PIN Demo ===\n");

    // 1. Create identities
    println!("1. Creating identities...");
    let server_identity = Arc::new(
        michi_identity::IdentityManager::generate(dir.path(), "Micro Server", "pw")
            .expect("server generation failed"),
    );
    let client_identity =
        michi_identity::IdentityManager::generate(dir.path(), "Mobile Client", "pw")
            .expect("client generation failed");

    println!("   Server Michi ID:  {}", server_identity.michi_id());
    println!("   Client Michi ID:  {}", client_identity.michi_id());

    // 2. Client builds a signed challenge
    println!("\n2. Client builds signed challenge...");
    let nonce: [u8; 16] = rand::random();
    let (challenge_signature, _) = client_identity.sign_base64url(&nonce);
    let request = michi_identity::PairStartRequest {
        device_name: "Michi Mobile".into(),
        device_type: "mobile".into(),
        roles: vec![
            michi_identity::Role::MobilePlayer,
            michi_identity::Role::RemoteController,
        ],
        auth_strategy: michi_identity::AuthStrategy::Ed25519Challenge,
        michi_id: client_identity.michi_id().to_base64url(),
        public_key: client_identity.public_key_base64url(),
        challenge_nonce: michi_identity::encode_base64url(&nonce),
        challenge_signature,
    };

    // 3. Server opens the session
    println!("\n3. Server starts pairing...");
    let registry = michi_identity::PairingRegistry::new();
    let (response, pin) = registry
        .start_server(&server_identity, &request, "192.168.1.50")
        .expect("start server failed");

    println!("   Session ID:    {}", response.session_id);
    println!("   Expires at:    {}", response.expires_at);
    println!("   Attempts:      {}", response.attempts_remaining);
    println!(
        "   Generated PIN: {} (shown locally, never sent over the wire)",
        pin
    );
    println!(
        "   Lifetime:      {} seconds",
        michi_identity::pairing::MAX_SESSION_DURATION.as_secs()
    );

    // 4. Client confirms with the correct PIN
    println!("\n4. Client confirms with correct PIN...");
    let confirm = michi_identity::PairConfirmRequest {
        session_id: response.session_id,
        pin,
        michi_id: client_identity.michi_id().to_base64url(),
        public_key: client_identity.public_key_base64url(),
    };
    match registry.confirm(&confirm, "192.168.1.50") {
        Ok(session) => println!(
            "   ✅ Pairing successful! Session consumed: {}",
            session.consumed
        ),
        Err(e) => println!("   ❌ Pairing failed: {}", e),
    }

    // 5. Reuse after success is rejected
    println!("\n5. Reusing the first session...");
    match registry.confirm(&confirm, "192.168.1.50") {
        Err(e) => println!("   ✅ Reuse rejected: {}", e),
        Ok(_) => println!("   ❌ Reuse accepted (should not happen)"),
    }

    // 6. Wrong PIN is rejected on a fresh session
    println!("\n6. Wrong PIN on a fresh session...");
    let (response2, _pin2) = registry
        .start_server(&server_identity, &request, "192.168.1.50")
        .expect("start server failed");
    let wrong = michi_identity::PairConfirmRequest {
        session_id: response2.session_id,
        pin: "000000".into(),
        michi_id: client_identity.michi_id().to_base64url(),
        public_key: client_identity.public_key_base64url(),
    };
    match registry.confirm(&wrong, "192.168.1.50") {
        Err(e) => println!("   ✅ Wrong PIN rejected: {}", e),
        Ok(_) => println!("   ❌ Wrong PIN accepted (should not happen)"),
    }

    println!("\n=== Demo complete ===");
}
