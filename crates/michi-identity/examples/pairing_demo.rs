/// Demostración completa de pairing TOFU + PIN.
///
/// Muestra:
/// 1. Dos identidades: Servidor y Cliente.
/// 2. Cliente crea challenge firmado.
/// 3. Servidor inicia pairing, genera PIN.
/// 4. Cliente envía PIN + prueba firmada.
/// 5. Servidor confirma.
/// 6. Verificación: PIN incorrecto debe fallar.
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    let dir = TempDir::new().expect("failed to create temp dir");

    println!("=== Michi Pairing TOFU + PIN Demo ===\n");

    // 1. Crear identidades
    println!("1. Creating identities...");
    let server_identity = michi_identity::IdentityManager::generate(dir.path(), "Micro Server")
        .await
        .expect("server generation failed");
    let client_identity = michi_identity::IdentityManager::generate(dir.path(), "Mobile Client")
        .await
        .expect("client generation failed");

    println!("   Server Michi ID:  {}", server_identity.michi_id());
    println!("   Client Michi ID:  {}", client_identity.michi_id());

    // 2. Cliente crea challenge firmado (prueba de posesión de sk)
    println!("\n2. Client creates signed challenge...");
    let client_challenge =
        michi_identity::PairingProtocol::create_challenge(&client_identity)
            .expect("create challenge failed");
    println!("   Challenge nonce: {}...", &client_challenge.nonce[..16]);

    // 3. Servidor verifica challenge e inicia pairing
    println!("\n3. Server starts pairing...");
    let (session, pin) = michi_identity::PairingProtocol::start_server(
        client_identity.public_key_bytes(),
        &client_challenge,
    )
    .expect("start server failed");

    println!("   Generated PIN:   {}", pin);
    println!("   Peer Michi ID:   {}", session.peer_michi_id);

    // 4. Cliente prepara confirmación
    println!("\n4. Client prepares confirmation...");
    let pin_proof = michi_identity::pairing::PairingProtocol::hash_pin(
        &pin,
        &session.server_nonce,
    );
    let (pin_proof_sig, _) = client_identity.sign_standard(&pin_proof);
    println!("   Pin proof hash:  {}...", hex::encode(&pin_proof[..4]));

    // 5. Servidor confirma pairing
    println!("\n5. Server confirms pairing...");
    let result = michi_identity::PairingProtocol::confirm_server(
        &session,
        &pin,
        &pin_proof_sig,
        client_identity.public_key_bytes(),
    );

    match result {
        Ok(()) => println!("   ✅ Pairing successful! TOFU complete."),
        Err(e) => println!("   ❌ Pairing failed: {}", e),
    }

    // 6. PIN incorrecto debe fallar
    println!("\n6. Verifying wrong PIN is rejected...");
    let result = michi_identity::PairingProtocol::confirm_server(
        &session,
        "000000",
        &pin_proof_sig,
        client_identity.public_key_bytes(),
    );

    match result {
        Ok(()) => println!("   ❌ Wrong PIN was accepted (should not happen)"),
        Err(e) => println!("   ✅ Wrong PIN correctly rejected: {}", e),
    }

    println!("\n=== Demo complete ===");
}
