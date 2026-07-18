use crate::error::IdentityError;
use crate::identity::IdentityManager;
use crate::types::{MichiId, PeerInfo};
use std::sync::Arc;

const URI_SCHEME: &str = "michi";
const URI_HOST: &str = "pair";

/// Conector QR para pairing fuera de banda.
pub struct QRConnector {
    identity: Arc<IdentityManager>,
}

impl QRConnector {
    pub fn new(identity: Arc<IdentityManager>) -> Self {
        Self { identity }
    }

    /// Genera una URI firmada para pairing.
    /// Formato: michi://pair?pk=<base64url>&id=<hex>&nonce=<base64url>&sig=<base64url>&name=<url>
    pub fn generate_pairing_uri(&self) -> Result<String, IdentityError> {
        let nonce: [u8; 16] = rand::random();
        let (sig_bytes, pk_bytes) = self.identity.sign_raw(&nonce);

        let pk_b64 = Self::to_b64url(&pk_bytes);
        let nonce_b64 = Self::to_b64url(&nonce);
        let sig_b64 = Self::to_b64url(&sig_bytes);
        let name_encoded = urlencoding::encode(self.identity.device_name());

        let uri = format!(
            "{}://{}?pk={}&id={}&nonce={}&sig={}&name={}",
            URI_SCHEME, URI_HOST, pk_b64, self.identity.michi_id().hex(), nonce_b64, sig_b64, name_encoded
        );
        Ok(uri)
    }

    /// Parsea una URI de pairing y verifica la firma.
    pub fn parse_pairing_uri(&self, uri: &str) -> Result<PeerInfo, IdentityError> {
        let parsed = url::Url::parse(uri).map_err(|e| {
            IdentityError::UriParseFailed(format!("invalid URI: {}", e))
        })?;

        if parsed.scheme() != URI_SCHEME {
            return Err(IdentityError::UriParseFailed(format!("invalid scheme: {}", parsed.scheme())));
        }
        if parsed.host_str() != Some(URI_HOST) {
            return Err(IdentityError::UriParseFailed(format!("invalid host: {:?}", parsed.host_str())));
        }

        let get_param = |name: &str| -> Result<String, IdentityError> {
            // `url` crate already percent-decodes query params
            parsed.query_pairs()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| IdentityError::UriParseFailed(format!("missing parameter: {}", name)))
        };

        let pk_b64 = get_param("pk")?;
        let id_hex = get_param("id")?;
        let nonce_b64 = get_param("nonce")?;
        let sig_b64 = get_param("sig")?;
        let device_name = get_param("name").unwrap_or_else(|_| "Unknown".into());

        // Usar URL_SAFE_NO_PAD decode directamente
        let nonce_bytes = Self::from_b64url(&nonce_b64)?;
        let sig_bytes = Self::from_b64url(&sig_b64)?;
        let pk_bytes = Self::from_b64url(&pk_b64)?;

        // Verificar usando raw bytes
        use ed25519_dalek::{Signature, VerifyingKey};
        let sig = Signature::from_slice(&sig_bytes)
            .map_err(|e| IdentityError::InvalidSignature(e.to_string()))?;
        let pk_arr: [u8; 32] = pk_bytes.clone().try_into()
            .map_err(|_| IdentityError::InvalidSignature("invalid pk length".into()))?;
        let pk = VerifyingKey::from_bytes(&pk_arr)
            .map_err(|e| IdentityError::InvalidSignature(e.to_string()))?;

        let valid = pk.verify_strict(&nonce_bytes, &sig).is_ok();
        if !valid {
            return Err(IdentityError::InvalidSignature("QR signature invalid".into()));
        }

        // Verificar michi_id
        let computed_michi_id = MichiId::from_public_key(&pk);
        if computed_michi_id.hex() != id_hex {
            return Err(IdentityError::UriParseFailed("michi_id does not match public key".into()));
        }

        Ok(PeerInfo {
            michi_id: computed_michi_id,
            public_key: pk_bytes,
            device_name,
        })
    }

    fn to_b64url(data: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    }

    fn from_b64url(data: &str) -> Result<Vec<u8>, IdentityError> {
        use base64::Engine;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityManager;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_generate_and_parse_uri() {
        let dir = TempDir::new().unwrap();
        let identity = Arc::new(IdentityManager::generate(dir.path(), "test-device").await.unwrap());
        let connector = QRConnector::new(identity.clone());

        let uri = connector.generate_pairing_uri().unwrap();
        assert!(uri.starts_with("michi://pair?"));

        let peer = connector.parse_pairing_uri(&uri).unwrap();
        assert_eq!(peer.michi_id.hex(), identity.michi_id().hex());
        assert_eq!(peer.device_name, "test-device");
    }

    #[tokio::test]
    async fn test_invalid_uri_fails() {
        let dir = TempDir::new().unwrap();
        let identity = Arc::new(IdentityManager::generate(dir.path(), "test-device").await.unwrap());
        let connector = QRConnector::new(identity);
        let result = connector.parse_pairing_uri("https://evil.com/pair");
        assert!(result.is_err());
    }
}
