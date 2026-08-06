//! Basic identity demonstration.
//!
//! Shows:
//! 1. Initialization (generate or load keys).
//! 2. Signing a message.
//! 3. Verifying a signature.
//! 4. Loading an existing identity (same keys, same michi_id).
//! 5. Wrong password rejection.
use tempfile::TempDir;

fn main() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_dir = dir.path();

    println!("=== Michi Identity Demo ===\n");

    // 1. Initialize identity
    println!("1. Initializing identity...");
    let (identity, _discovery) = michi_identity::init(config_dir, "Demo Device", "demo-password")
        .expect("failed to init identity");

    println!("   Device name:   {}", identity.device_name());
    println!("   Michi ID:      {}", identity.michi_id());
    println!("   Public key:    {}", identity.public_key_base64url());
    println!("   Created at:    {}", identity.created_at());

    // 2. Sign a message
    println!("\n2. Signing message...");
    let message = b"Hello, Michi ecosystem!";
    let (signature, public_key) = identity.sign_base64url(message);
    println!("   Message:       {}", String::from_utf8_lossy(message));
    println!("   Signature:     {}...", &signature[..20]);

    // 3. Verify the signature
    println!("\n3. Verifying signature...");
    let valid = michi_identity::IdentityManager::verify(message, &signature, &public_key)
        .expect("verify failed");
    println!("   Valid:         {}", valid);

    // 4. Load the identity again: same michi_id (keys persisted)
    println!("\n4. Reloading identity (should be same)...");
    let (identity2, _) =
        michi_identity::init(config_dir, "Demo Device", "demo-password").expect("failed to reload");
    println!(
        "   Same Michi ID: {}",
        identity.michi_id() == identity2.michi_id()
    );

    // 5. Wrong password is rejected, identity is NOT regenerated
    println!("\n5. Wrong password...");
    match michi_identity::init(config_dir, "Demo Device", "wrong-password") {
        Ok(_) => println!("   ❌ wrong password was accepted (should not happen)"),
        Err(e) => println!("   ✅ rejected: {}", e),
    }

    println!("\n=== Demo complete ===");
}
