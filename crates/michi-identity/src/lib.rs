//! michi-identity: Sistema de Identidad y Confianza Descentralizado para el ecosistema Michi.
//!
//! Basado en criptografía Ed25519, permite que los dispositivos se reconozcan
//! por su identidad única (michi_id) en lugar de IP. Incluye:
//!
//! - `IdentityManager`: generación y persistencia de pares de claves Ed25519.
//! - `DiscoveryEngine`: anuncio y descubrimiento con announces firmados.
//! - `PairingProtocol`: emparejamiento TOFU + PIN de 6 dígitos.
//! - `QRConnector`: URIs firmadas para pairing fuera de banda.
//!
//! ## Retrocompatibilidad
//!
//! Todos los cambios son aditivos. Los announces legacy (sin firma) se aceptan
//! como `untrusted`. `michi_id: null` indica que un dispositivo no tiene
//! identidad Ed25519 inicializada. `api_version: "v1"` se mantiene.
//!
//! ## Uso rápido
//!
//! ```rust,no_run
//! use michi_identity::init;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config_dir = std::path::Path::new("/home/user/.config/michi");
//!     let (identity, discovery) = init(config_dir, "Mi Dispositivo").await.unwrap();
//!     println!("Michi ID: {}", identity.michi_id());
//! }
//! ```

pub mod discovery;
pub mod error;
pub mod identity;
pub mod pairing;
pub mod qr;
pub mod types;

use std::sync::Arc;

pub use discovery::DiscoveryEngine;
pub use error::IdentityError;
pub use identity::IdentityManager;
pub use pairing::PairingProtocol;
pub use qr::QRConnector;
pub use types::{
    AuthChallenge, AuthStrategy, IdentityDocument, MichiId, PairingSession, PeerInfo,
    SignedAnnounce, TrustLevel,
};

/// Inicializa el sistema de identidad completo.
///
/// 1. Carga o genera el par de claves Ed25519.
/// 2. Crea el DiscoveryEngine.
///
/// Este es el punto de entrada recomendado para Player, Micro Server, etc.
pub async fn init(
    config_dir: &std::path::Path,
    device_name: &str,
) -> Result<(Arc<IdentityManager>, DiscoveryEngine), IdentityError> {
    let identity = Arc::new(
        IdentityManager::load_or_generate(config_dir, device_name).await?,
    );
    let discovery = DiscoveryEngine::new(identity.clone());
    Ok((identity, discovery))
}
