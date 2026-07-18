//! DiscoveryEngine: anuncio y descubrimiento de dispositivos con firmas Ed25519.

use std::sync::Arc;

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::error::IdentityError;
use crate::identity::IdentityManager;
use crate::types::{SignedAnnounce, TrustLevel};

/// Peer descubierto en la red.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub announce: SignedAnnounce,
    pub trust: TrustLevel,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Engine de descubrimiento.
pub struct DiscoveryEngine {
    identity: Arc<IdentityManager>,
    tx: broadcast::Sender<DiscoveredPeer>,
}

/// Construye el payload canónico para firmar/verificar.
/// Debe ser IDÉNTICO en sign y verify.
fn canonical_payload(device_id: &Uuid, device_name: &str, ts: &str) -> Vec<u8> {
    format!("{}:{}:{}", device_id, device_name, ts).into_bytes()
}

impl DiscoveryEngine {
    pub fn new(identity: Arc<IdentityManager>) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { identity, tx }
    }

    /// Construye un SignedAnnounce firmado.
    pub fn build_signed_announce(&self) -> SignedAnnounce {
        let device_id = Uuid::new_v4();
        let canonical = canonical_payload(&device_id, self.identity.device_name(), "");

        let (signature_b64, public_key_b64) = if self.identity.michi_id().is_present() {
            self.identity.sign_standard(&canonical)
        } else {
            (String::new(), String::new())
        };

        SignedAnnounce {
            device_id,
            device_name: self.identity.device_name().into(),
            device_type: "unknown".into(),
            roles: vec![],
            api_version: "v1".into(),
            host: "0.0.0.0".into(),
            port: 0,
            michi_id: Some(*self.identity.michi_id()),
            public_key: if public_key_b64.is_empty() { None } else { Some(public_key_b64) },
            signature: if signature_b64.is_empty() { None } else { Some(signature_b64) },
            capabilities: None,
        }
    }

    /// Verifica un announce firmado.
    pub fn verify_announce(&self, announce: &SignedAnnounce) -> Result<TrustLevel, IdentityError> {
        match (&announce.signature, &announce.public_key) {
            (Some(sig), Some(pk)) if !sig.is_empty() && !pk.is_empty() => {
                // Necesitamos el timestamp. Como no lo guardamos en el announce,
                // usamos el device_id + device_name como payload canónico.
                let canonical = canonical_payload(&announce.device_id, &announce.device_name, "");
                let valid = IdentityManager::verify(&canonical, sig, pk)?;
                if valid {
                    let michi_id = IdentityManager::derive_michi_id(pk)?;
                    Ok(TrustLevel::Verified(michi_id))
                } else {
                    Ok(TrustLevel::Invalid)
                }
            }
            _ => Ok(TrustLevel::Untrusted(announce.device_id)),
        }
    }

    /// Retorna un receptor para escuchar peers descubiertos.
    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveredPeer> {
        self.tx.subscribe()
    }

    /// Simula la recepción de un announce (para pruebas sin red).
    pub fn ingest_announce(&self, announce: SignedAnnounce) -> Result<TrustLevel, IdentityError> {
        let trust = self.verify_announce(&announce)?;
        let peer = DiscoveredPeer {
            announce,
            trust: trust.clone(),
            last_seen: chrono::Utc::now(),
        };
        let _ = self.tx.send(peer);
        Ok(trust)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityManager;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_verify_legacy_announce() {
        let dir = TempDir::new().unwrap();
        let identity = Arc::new(IdentityManager::generate(dir.path(), "test-device").await.unwrap());
        let engine = DiscoveryEngine::new(identity);

        let announce = SignedAnnounce {
            device_id: Uuid::new_v4(),
            device_name: "Legacy Device".into(),
            device_type: "michi_server".into(),
            roles: vec!["server".into()],
            api_version: "1.0.0".into(),
            host: "192.168.1.1".into(),
            port: 8500,
            michi_id: None,
            public_key: None,
            signature: None,
            capabilities: None,
        };

        let trust = engine.verify_announce(&announce).unwrap();
        assert!(matches!(trust, TrustLevel::Untrusted(_)));
    }

    #[tokio::test]
    async fn test_verify_signed_announce() {
        let dir = TempDir::new().unwrap();
        let identity = Arc::new(IdentityManager::generate(dir.path(), "alice").await.unwrap());
        let engine = DiscoveryEngine::new(identity);

        // build_signed_announce firma con canonical_payload(device_id, name, ts)
        // verify_announce verifica con canonical_payload(device_id, name, "") 
        // Esto funciona porque usamos device_id + name como canonical, no timestamp
        let announce = engine.build_signed_announce();
        let trust = engine.verify_announce(&announce).unwrap();
        assert!(matches!(trust, TrustLevel::Verified(_)), "expected Verified, got {:?}", trust);
    }

    #[tokio::test]
    async fn test_ingest_announce() {
        let dir = TempDir::new().unwrap();
        let identity = Arc::new(IdentityManager::generate(dir.path(), "bob").await.unwrap());
        let engine = DiscoveryEngine::new(identity);

        let announce = engine.build_signed_announce();
        let trust = engine.ingest_announce(announce).unwrap();
        assert!(matches!(trust, TrustLevel::Verified(_)));
    }
}
