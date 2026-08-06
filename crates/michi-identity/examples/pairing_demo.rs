//! Full TOFU + PIN pairing demonstration.
//!
//! Shows:
//! 1. Server and client identities.
//! 2. Server opens a pairing session (5-minute lifetime, 5 attempts).
//! 3. Client confirms with the correct PIN (constant-time verification).
//! 4. Wrong PIN is rejected and consumes an attempt.
//! 5. Session reuse after success is rejected.
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
    let client_identity = Arc::new(
        michi_identity::IdentityManager::generate(dir.path(), "Mobile Client", "pw")
            .expect("client generation failed"),
    );

    println!("   Server Michi ID:  {}", server_identity.michi_id());
    println!("   Client Michi ID:  {}", client_identity.michi_id());

    // 2. Server opens a pairing session
    println!("\n2. Server starts pairing...");
    let registry = michi_identity::PairingRegistry::new();
    let client_pk = client_identity.public_key_bytes().to_vec();
    let (session_id, pin) = registry
        .start_server(
            &server_identity,
            &client_pk,
            Some(&client_identity.michi_id().to_base64url()),
        )
        .expect("start server failed");

    println!("   Session ID:    {}", session_id);
    println!("   Generated PIN: {}", pin);
    println!(
        "   Lifetime:      {} seconds",
        michi_identity::pairing::MAX_SESSION_DURATION.as_secs()
    );
    println!(
        "   Max attempts:  {}",
        michi_identity::pairing::MAX_ATTEMPTS
    );

    // 3. Client confirms with the correct PIN
    println!("\n3. Client confirms with correct PIN...");
    let result = registry.confirm(
        session_id,
        &pin,
        &client_pk,
        Some(&client_identity.michi_id().to_base64url()),
    );
    match result {
        Ok(()) => println!("   ✅ Pairing successful! TOFU complete."),
        Err(e) => println!("   ❌ Pairing failed: {}", e),
    }

    // 4. Wrong PIN is rejected (a new session is needed)
    println!("\n4. Wrong PIN on a fresh session...");
    let (session_id2, _pin2) = registry
        .start_server(&server_identity, &client_pk, None)
        .expect("start server failed");
    let result = registry.confirm(session_id2, "000000", &client_pk, None);
    match result {
        Err(e) => println!("   ✅ Wrong PIN rejected: {}", e),
        Ok(()) => println!("   ❌ Wrong PIN accepted (should not happen)"),
    }

    // 5. Session reuse after success is rejected
    println!("\n5. Reusing the first session...");
    let result = registry.confirm(
        session_id,
        &pin,
        &client_pk,
        Some(&client_identity.michi_id().to_base64url()),
    );
    match result {
        Err(e) => println!("   ✅ Reuse rejected: {}", e),
        Ok(()) => println!("   ❌ Reuse accepted (should not happen)"),
    }

    println!("\n=== Demo complete ===");
}
