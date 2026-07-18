use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// MichiId: identidad derivada criptográficamente.
///
/// Se obtiene como blake3(public_key_ed25519).
/// Puede ser None para dispositivos legacy (sin identidad Ed25519).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MichiId(#[serde(with = "michi_id_serde")] pub Option<[u8; 32]>);

impl MichiId {
    pub fn from_public_key(pk: &VerifyingKey) -> Self {
        let hash = blake3::hash(&pk.to_bytes());
        Self(Some(*hash.as_bytes()))
    }

    pub fn null() -> Self {
        Self(None)
    }

    pub fn is_present(&self) -> bool {
        self.0.is_some()
    }

    /// Retorna los 32 bytes del hash, o None si es null.
    pub fn as_bytes(&self) -> Option<&[u8; 32]> {
        self.0.as_ref()
    }

    /// Retorna el hex del hash, o "null" si es None.
    pub fn hex(&self) -> String {
        match self.0 {
            Some(ref b) => hex::encode(b),
            None => "null".to_string(),
        }
    }
}

impl fmt::Display for MichiId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(ref b) => write!(f, "{}", hex::encode(b)),
            None => write!(f, "null"),
        }
    }
}

impl From<&VerifyingKey> for MichiId {
    fn from(pk: &VerifyingKey) -> Self {
        Self::from_public_key(pk)
    }
}

mod michi_id_serde {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(opt: &Option<[u8; 32]>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt {
            Some(ref b) => s.serialize_str(&hex::encode(b)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<[u8; 32]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(d)?;
        match s {
            Some(ref hex_str) if hex_str == "null" => Ok(None),
            Some(ref hex_str) => {
                let bytes = hex::decode(hex_str).map_err(D::Error::custom)?;
                if bytes.len() != 32 {
                    return Err(D::Error::custom("michi_id must be 32 bytes (64 hex chars)"));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(Some(arr))
            }
            None => Ok(None),
        }
    }
}

/// Nivel de confianza de un peer descubierto.
#[derive(Debug, Clone)]
pub enum TrustLevel {
    /// Firma Ed25519 válida. Contiene el MichiId del peer.
    Verified(MichiId),
    /// Sin firma (legacy UUID). Se acepta pero sin verificación.
    Untrusted(Uuid),
    /// Firma presente pero inválida. No confiar.
    Invalid,
}

/// Estrategia de autenticación negociada durante pairing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthStrategy {
    #[serde(rename = "PLAYER_PASSWORD")]
    PlayerPassword,
    #[serde(rename = "SERVER_CODE")]
    ServerCode,
    #[serde(rename = "ED25519_CHALLENGE")]
    Ed25519Challenge,
    #[serde(rename = "RECEIVER_BUTTON")]
    ReceiverButton,
    #[serde(rename = "LEGACY")]
    Legacy,
}

/// Payload firmado para announces de discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAnnounce {
    // --- Legacy fields (siempre presentes) ---
    pub device_id: Uuid,
    pub device_name: String,
    pub device_type: String,
    pub roles: Vec<String>,
    pub api_version: String,
    pub host: String,
    pub port: u16,

    // --- Identity fields (opcionales) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub michi_id: Option<MichiId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    // --- Capabilities ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
}

/// Challenge criptográfico para pairing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub michi_id: MichiId,
    /// Nonce aleatorio en base64.
    pub nonce: String,
    /// Firma Ed25519 del nonce en base64.
    pub signature: String,
}

/// Información de un peer obtenida desde QR.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub michi_id: MichiId,
    pub public_key: Vec<u8>,
    pub device_name: String,
}

/// Sesión de pairing activa.
#[derive(Debug, Clone)]
pub struct PairingSession {
    pub peer_michi_id: MichiId,
    pub peer_public_key: Vec<u8>,
    pub pin: String,
    pub pin_hash: [u8; 32],
    pub server_nonce: Vec<u8>,
    pub client_nonce: Vec<u8>,
}

/// Documento de identidad para server-info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityDocument {
    pub michi_id: MichiId,
    pub public_key: String,
    pub algorithm: String,
    pub device_name: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_challenge: Option<AuthChallenge>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_michi_id_from_pk() {
        use ed25519_dalek::VerifyingKey;
        let pk = VerifyingKey::from_bytes(&[0u8; 32]).unwrap();
        let mid = MichiId::from_public_key(&pk);
        assert!(mid.is_present());
        assert_eq!(mid.as_bytes().unwrap().len(), 32);
    }

    #[test]
    fn test_michi_id_null() {
        let mid = MichiId::null();
        assert!(!mid.is_present());
        assert_eq!(mid.hex(), "null");
    }

    #[test]
    fn test_michi_id_serde_roundtrip() {
        let mid = MichiId::null();
        let json = serde_json::to_string(&mid).unwrap();
        assert_eq!(json, "null");
        let des: MichiId = serde_json::from_str(&json).unwrap();
        assert!(!des.is_present());
    }

    #[test]
    fn test_auth_strategy_serde() {
        let s = AuthStrategy::Ed25519Challenge;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"ED25519_CHALLENGE\"");
        let des: AuthStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(des, AuthStrategy::Ed25519Challenge);
    }
}
