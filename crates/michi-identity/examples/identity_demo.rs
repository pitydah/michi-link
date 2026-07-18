/// Demostración básica del sistema de identidad.
///
/// Muestra:
/// 1. Inicialización (generar o cargar claves).
/// 2. Firmar un mensaje.
/// 3. Verificar una firma.
/// 4. Obtener el michi_id.
use std::path::Path;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    // Usamos un directorio temporal para la demo
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_dir = dir.path();

    println!("=== Michi Identity Demo ===\n");

    // 1. Inicializar identidad
    println!("1. Initializing identity...");
    let (identity, _discovery) =
        michi_identity::init(config_dir, "Demo Device")
            .await
            .expect("failed to init identity");

    println!("   Device name:   {}", identity.device_name());
    println!("   Michi ID:      {}", identity.michi_id());
    println!("   Public key:    {}", identity.public_key_b64());

    // 2. Firmar un mensaje
    println!("\n2. Signing message...");
    let message = b"Hola, ecosistema Michi!";
    let (signature, public_key) = identity.sign_standard(message);
    println!("   Message:       {}", String::from_utf8_lossy(message));
    println!("   Signature:     {}...", &signature[..20]);

    // 3. Verificar la firma
    println!("\n3. Verifying signature...");
    let valid = michi_identity::IdentityManager::verify(message, &signature, &public_key)
        .expect("verify failed");
    println!("   Valid:         {}", valid);

    // 4. Verificar que una firma incorrecta falla
    let wrong_message = b"Mensaje incorrecto";
    let invalid = michi_identity::IdentityManager::verify(wrong_message, &signature, &public_key)
        .expect("verify failed");
    println!("   Wrong msg:     {}", invalid);

    // 5. Cargar identidad existente (debe ser la misma)
    println!("\n4. Reloading identity (should be same)...");
    let (identity2, _) = michi_identity::init(config_dir, "Demo Device")
        .await
        .expect("failed to reload");
    println!("   Same Michi ID: {}", identity.michi_id().hex() == identity2.michi_id().hex());

    println!("\n=== Demo complete ===");
}
