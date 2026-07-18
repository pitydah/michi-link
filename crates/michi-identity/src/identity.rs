//! IdentityManager: generación, persistencia y operaciones con claves Ed25519.
//!
//! ## Flujo criptográfico
//!
//! ```text
//! OS RNG ──→ ed25519-dalek ──→ Keypair
//!                                 │
//!                                 ├── secret_key → cifrado → identity.msgpack
//!                                 │                    (blake3(hostname+salt))
//!                                 │
//!                                 └── public_key ──→ blake3 ──→ michi_id
//!                                                                 (32 bytes hex)
//! ```

use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::error::IdentityError;
use crate::types::MichiId;

const IDENTITY_FILE: &str = "identity.msgpack";
const WRAP_KEY_CONTEXT: &[u8] = b"michi-identity-key-wrap-v1";

/// Archivo de identidad persistido en disco (MessagePack).
#[derive(Debug, Serialize, Deserialize)]
struct IdentityFile {
    version: u32,
    algorithm: String,
    created_at: String,
    device_name: String,
    wrapped_secret_key: Vec<u8>,
    public_key: Vec<u8>,
    michi_id: [u8; 32],
    salt: [u8; 16],
}

/// Administrador de identidad del dispositivo.
pub struct IdentityManager {
    signer: SigningKey,
    verifying_key: VerifyingKey,
    michi_id: MichiId,
    file_path: PathBuf,
    device_name: String,
}

impl IdentityManager {
    /// Genera un nuevo par de claves y lo persiste.
    pub async fn generate(config_dir: &Path, device_name: &str) -> Result<Self, IdentityError> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let michi_id = MichiId::from_public_key(&verifying_key);

        let (wrapped_secret_key, salt) = Self::wrap_key(signing_key.to_bytes())?;

        let file_path = config_dir.join(IDENTITY_FILE);
        let now = chrono::Utc::now().to_rfc3339();

        let file = IdentityFile {
            version: 1,
            algorithm: "ed25519".into(),
            created_at: now,
            device_name: device_name.to_string(),
            wrapped_secret_key,
            public_key: verifying_key.to_bytes().to_vec(),
            michi_id: *michi_id.as_bytes().ok_or_else(|| {
                IdentityError::Internal("michi_id should be present after generation".into())
            })?,
            salt,
        };

        Self::write_file(&file_path, &file)?;

        tracing::info!("Identity generated: michi_id={}", michi_id);

        Ok(Self {
            signer: signing_key,
            verifying_key,
            michi_id,
            file_path,
            device_name: device_name.to_string(),
        })
    }

    /// Carga desde disco o genera si no existe.
    pub async fn load_or_generate(config_dir: &Path, device_name: &str) -> Result<Self, IdentityError> {
        let file_path = config_dir.join(IDENTITY_FILE);

        if file_path.exists() {
            match Self::load(&file_path).await {
                Ok(mgr) => return Ok(mgr),
                Err(e) => {
                    tracing::warn!("Failed to load identity, generating new: {}", e);
                }
            }
        }

        Self::generate(config_dir, device_name).await
    }

    /// Carga identidad desde disco.
    async fn load(file_path: &Path) -> Result<Self, IdentityError> {
        let data = tokio::fs::read(file_path).await?;
        let file: IdentityFile = rmp_serde::from_slice(&data)?;

        if file.algorithm != "ed25519" {
            return Err(IdentityError::KeyLoadFailed(format!("Unknown algorithm: {}", file.algorithm)));
        }

        let secret_key = Self::unwrap_key(&file.wrapped_secret_key, &file.salt)?;
        let arr: [u8; 32] = secret_key.try_into().map_err(|_| {
            IdentityError::KeyLoadFailed("invalid secret key length".into())
        })?;
        let signer = SigningKey::from_bytes(&arr);
        let verifying_key = signer.verifying_key();

        let computed_michi_id = blake3::hash(&verifying_key.to_bytes());
        let stored_id = &file.michi_id;
        if computed_michi_id.as_bytes() != stored_id {
            return Err(IdentityError::KeyLoadFailed("michi_id mismatch".into()));
        }

        let michi_id = MichiId(Some(*stored_id));

        tracing::info!("Identity loaded: michi_id={}", michi_id);

        Ok(Self {
            signer,
            verifying_key,
            michi_id,
            file_path: file_path.to_path_buf(),
            device_name: file.device_name,
        })
    }

    /// Firma un payload y retorna (signature_base64_standard, public_key_base64_standard).
    pub fn sign_standard(&self, payload: &[u8]) -> (String, String) {
        use base64::Engine;
        let sig = self.signer.sign(payload);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        let pk_b64 = base64::engine::general_purpose::STANDARD.encode(self.verifying_key.to_bytes());
        (sig_b64, pk_b64)
    }

    /// Firma un payload y retorna (signature_bytes_raw, public_key_bytes_raw).
    pub fn sign_raw(&self, payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let sig = self.signer.sign(payload);
        (sig.to_bytes().to_vec(), self.verifying_key.to_bytes().to_vec())
    }

    /// Verifica una firma Ed25519.
    ///
    /// Acepta base64 STANDARD y URL_SAFE_NO_PAD.
    pub fn verify(payload: &[u8], signature_b64: &str, public_key_b64: &str) -> Result<bool, IdentityError> {
        use base64::Engine;
        let sig_bytes = Self::decode_b64(signature_b64)?;
        let pk_bytes = Self::decode_b64(public_key_b64)?;

        let sig = Signature::from_slice(&sig_bytes)?;
        let pk = VerifyingKey::from_bytes(&pk_bytes.try_into().map_err(|_| {
            IdentityError::InvalidSignature("public key must be 32 bytes".into())
        })?)?;

        Ok(pk.verify_strict(payload, &sig).is_ok())
    }

    /// Deriva un MichiId desde una public key base64 (STANDARD o URL_SAFE).
    pub fn derive_michi_id(public_key_b64: &str) -> Result<MichiId, IdentityError> {
        let pk_bytes = Self::decode_b64(public_key_b64)?;
        let pk = VerifyingKey::from_bytes(&pk_bytes.try_into().map_err(|_| {
            IdentityError::InvalidSignature("public key must be 32 bytes".into())
        })?)?;
        Ok(MichiId::from_public_key(&pk))
    }

    /// Retorna el MichiId de este dispositivo.
    pub fn michi_id(&self) -> &MichiId {
        &self.michi_id
    }

    /// Retorna la public key en base64.
    pub fn public_key_b64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self.verifying_key.to_bytes())
    }

    /// Retorna la public key en bytes.
    pub fn public_key_bytes(&self) -> &[u8] {
        self.verifying_key.as_bytes()
    }

    /// Retorna el nombre del dispositivo.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Genera un IdentityDocument para incluir en server-info.
    pub fn identity_document(&self) -> crate::types::IdentityDocument {
        crate::types::IdentityDocument {
            michi_id: self.michi_id,
            public_key: self.public_key_b64(),
            algorithm: "ed25519".into(),
            device_name: self.device_name.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            auth_challenge: None,
        }
    }

    // --- Métodos privados ---

    fn wrap_key(secret_key: [u8; 32]) -> Result<(Vec<u8>, [u8; 16]), IdentityError> {
        let salt = {
            let mut s = [0u8; 16];
            use rand::RngCore;
            OsRng.fill_bytes(&mut s);
            s
        };
        let wrap_key = Self::derive_wrap_key(&salt);
        let wrapped: Vec<u8> = secret_key.iter().zip(wrap_key.iter().cycle()).map(|(k, w)| k ^ w).collect();
        Ok((wrapped, salt))
    }

    fn unwrap_key(wrapped: &[u8], salt: &[u8; 16]) -> Result<Vec<u8>, IdentityError> {
        let wrap_key = Self::derive_wrap_key(salt);
        let secret: Vec<u8> = wrapped.iter().zip(wrap_key.iter().cycle()).map(|(w, k)| w ^ k).collect();
        Ok(secret)
    }

    fn derive_wrap_key(salt: &[u8; 16]) -> [u8; 32] {
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        let mut hasher = blake3::Hasher::new();
        hasher.update(WRAP_KEY_CONTEXT);
        hasher.update(hostname.as_bytes());
        hasher.update(salt);
        *hasher.finalize().as_bytes()
    }

    fn decode_b64(data: &str) -> Result<Vec<u8>, IdentityError> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data))
            .map_err(|e| IdentityError::InvalidSignature(format!("invalid base64: {}", e)))
    }

    fn write_file(path: &Path, file: &IdentityFile) -> Result<(), IdentityError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = rmp_serde::to_vec(file)?;
        std::fs::write(path, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_generate_and_load() {
        let dir = TempDir::new().unwrap();
        let mgr = IdentityManager::generate(dir.path(), "test-device").await.unwrap();
        assert!(mgr.michi_id().is_present());
        assert_eq!(mgr.device_name(), "test-device");

        let loaded = IdentityManager::load(&dir.path().join(IDENTITY_FILE)).await.unwrap();
        assert_eq!(mgr.michi_id().hex(), loaded.michi_id().hex());
    }

    #[tokio::test]
    async fn test_load_or_generate() {
        let dir = TempDir::new().unwrap();
        let mgr = IdentityManager::load_or_generate(dir.path(), "test").await.unwrap();
        let mgr2 = IdentityManager::load_or_generate(dir.path(), "test").await.unwrap();
        assert_eq!(mgr.michi_id().hex(), mgr2.michi_id().hex());
    }

    #[test]
    fn test_sign_and_verify() {
        use rand::rngs::OsRng;
        let mut csprng = OsRng;
        let sk = SigningKey::generate(&mut csprng);
        let vk = sk.verifying_key();
        let mgr = IdentityManager {
            signer: sk,
            verifying_key: vk,
            michi_id: MichiId::null(),
            file_path: PathBuf::from("/tmp/test"),
            device_name: "test".into(),
        };

        let payload = b"hello world";
        let (sig, pk) = mgr.sign_standard(payload);
        let valid = IdentityManager::verify(payload, &sig, &pk).unwrap();
        assert!(valid);

        let invalid = IdentityManager::verify(b"wrong", &sig, &pk).unwrap();
        assert!(!invalid);
    }
}
