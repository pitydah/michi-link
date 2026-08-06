//! IdentityManager: generation, persistence and operations with Ed25519 keys.
//!
//! ## Cryptographic flow (contract v1)
//!
//! ```text
//! OS RNG ──→ ed25519-dalek ──→ Keypair
//!                                 │
//!                                 ├── secret_key → ChaCha20-Poly1305 (AEAD)
//!                                 │      key  = blake3(context || salt || password)
//!                                 │      AAD  = context || identity_scheme
//!                                 │      └── identity.msgpack (format_version 2)
//!                                 │
//!                                 └── public_key ──→ blake3 ──→ michi_id
//!                                                          (base64url, 43 chars)
//! ```
//!
//! ## Guarantees
//!
//! - At-rest encryption is authenticated (ChaCha20-Poly1305) with explicit AAD
//!   binding the ciphertext to the format and identity scheme.
//! - Writes are atomic (temp file + rename) with `0600` permissions on Unix.
//! - A corrupted or unauthenticated file returns an explicit error and NEVER
//!   silently regenerates the identity.
//! - Legacy (v1, XOR obfuscation) files are migrated automatically without
//!   generating a new identity.
//!
//! ## Recovery procedure
//!
//! If the identity file is corrupted, the operator must restore it from a
//! backup. Identity rotation is an explicit, user-initiated action:
//! delete the file and call `load_or_generate` — the library never does this
//! implicitly.

use std::path::Path;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::error::IdentityError;
use crate::types::MichiId;

const IDENTITY_FILE: &str = "identity.msgpack";
const FORMAT_VERSION: u32 = 2;
const IDENTITY_SCHEME: &str = "ed25519-blake3-v1";
/// Context binding the AEAD key derivation to this file format.
const FILE_CONTEXT: &[u8] = b"michi-identity-file-v2";
/// Legacy (v1) wrap key context, used only for migration.
const LEGACY_WRAP_CONTEXT: &[u8] = b"michi-identity-key-wrap-v1";

/// Identity file persisted on disk (MessagePack, format v2).
#[derive(Debug, Serialize, Deserialize)]
struct IdentityFileV2 {
    format_version: u32,
    identity_scheme: String,
    michi_id: String,
    public_key: String,
    encrypted_secret: String,
    salt: String,
    nonce: String,
    created_at: String,
    device_name: String,
}

/// Legacy (v1) identity file — accepted only for migration.
#[derive(Debug, Serialize, Deserialize)]
struct IdentityFileV1 {
    version: u32,
    algorithm: String,
    created_at: String,
    device_name: String,
    wrapped_secret_key: Vec<u8>,
    public_key: Vec<u8>,
    michi_id: [u8; 32],
    salt: [u8; 16],
}

/// Device identity manager.
#[derive(Debug)]
pub struct IdentityManager {
    signer: SigningKey,
    verifying_key: VerifyingKey,
    michi_id: MichiId,
    device_name: String,
    created_at: String,
}

impl IdentityManager {
    /// Generates a new keypair and persists it (atomic, AEAD-encrypted).
    pub fn generate(
        config_dir: &Path,
        device_name: &str,
        password: &str,
    ) -> Result<Self, IdentityError> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let michi_id = MichiId::from_public_key(&verifying_key);
        let created_at = chrono::Utc::now().to_rfc3339();

        let file_path = config_dir.join(IDENTITY_FILE);
        let file = Self::encrypt_v2(
            &signing_key,
            &verifying_key,
            device_name,
            &created_at,
            password,
        )?;

        Self::write_file(&file_path, &file)?;

        tracing::info!("Identity generated: michi_id={}", michi_id);

        Ok(Self {
            signer: signing_key,
            verifying_key,
            michi_id,
            device_name: device_name.to_string(),
            created_at,
        })
    }

    /// Loads the identity from disk with the given password.
    ///
    /// Migrates legacy (v1) files automatically. Returns an explicit error on
    /// corruption, wrong password or tampering — it never regenerates.
    pub fn load(config_dir: &Path, password: &str) -> Result<Self, IdentityError> {
        let file_path = config_dir.join(IDENTITY_FILE);
        Self::load_from_path(&file_path, password)
    }

    fn load_from_path(file_path: &Path, password: &str) -> Result<Self, IdentityError> {
        if !file_path.exists() {
            return Err(IdentityError::KeyNotFound(file_path.display().to_string()));
        }
        let data = std::fs::read(file_path)?;

        // Try the canonical v2 format first.
        if let Ok(file) = rmp_serde::from_slice::<IdentityFileV2>(&data) {
            return Self::decrypt_v2(&file, password);
        }

        // Then the legacy v1 format (migration path).
        if let Ok(legacy) = rmp_serde::from_slice::<IdentityFileV1>(&data) {
            tracing::info!("Legacy identity format detected, migrating");
            return Self::migrate_v1_to_v2(&legacy, file_path, password);
        }

        Err(IdentityError::IdentityCorrupted(format!(
            "unrecognized identity file format: {}",
            file_path.display()
        )))
    }

    /// Loads the identity or generates a new one — ONLY when the file is
    /// missing. Any corruption, password or integrity error is propagated.
    pub fn load_or_generate(
        config_dir: &Path,
        device_name: &str,
        password: &str,
    ) -> Result<Self, IdentityError> {
        let file_path = config_dir.join(IDENTITY_FILE);
        if file_path.exists() {
            return Self::load_from_path(&file_path, password);
        }
        Self::generate(config_dir, device_name, password)
    }

    /// Signs a payload and returns (signature_base64, public_key_base64).
    pub fn sign_standard(&self, payload: &[u8]) -> (String, String) {
        use base64::Engine;
        let sig = self.signer.sign(payload);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        let pk_b64 =
            base64::engine::general_purpose::STANDARD.encode(self.verifying_key.to_bytes());
        (sig_b64, pk_b64)
    }

    /// Signs a payload and returns (signature_bytes_raw, public_key_bytes_raw).
    pub fn sign_raw(&self, payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let sig = self.signer.sign(payload);
        (
            sig.to_bytes().to_vec(),
            self.verifying_key.to_bytes().to_vec(),
        )
    }

    /// Verifies an Ed25519 signature. Accepts base64 STANDARD and URL_SAFE_NO_PAD.
    pub fn verify(
        payload: &[u8],
        signature_b64: &str,
        public_key_b64: &str,
    ) -> Result<bool, IdentityError> {
        let sig_bytes = Self::decode_b64(signature_b64)?;
        let pk_bytes = Self::decode_b64(public_key_b64)?;

        let sig = Signature::from_slice(&sig_bytes)?;
        let pk =
            VerifyingKey::from_bytes(&pk_bytes.try_into().map_err(|_| {
                IdentityError::InvalidSignature("public key must be 32 bytes".into())
            })?)?;

        Ok(pk.verify_strict(payload, &sig).is_ok())
    }

    /// Derives a MichiId from a base64 public key (STANDARD or URL_SAFE).
    pub fn derive_michi_id(public_key_b64: &str) -> Result<MichiId, IdentityError> {
        let pk_bytes = Self::decode_b64(public_key_b64)?;
        let pk =
            VerifyingKey::from_bytes(&pk_bytes.try_into().map_err(|_| {
                IdentityError::InvalidSignature("public key must be 32 bytes".into())
            })?)?;
        Ok(MichiId::from_public_key(&pk))
    }

    /// Returns the MichiId of this device.
    pub fn michi_id(&self) -> &MichiId {
        &self.michi_id
    }

    /// Returns the public key in base64 (STANDARD).
    pub fn public_key_b64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self.verifying_key.to_bytes())
    }

    /// Returns the public key in raw bytes.
    pub fn public_key_bytes(&self) -> &[u8] {
        self.verifying_key.as_bytes()
    }

    /// Returns the device name.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Returns the identity creation timestamp (RFC 3339).
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Builds an IdentityDocument for `server/info`.
    pub fn identity_document(&self) -> crate::types::IdentityDocument {
        crate::types::IdentityDocument {
            michi_id: self.michi_id,
            public_key: self.public_key_b64(),
            identity_scheme: IDENTITY_SCHEME.to_string(),
            device_name: self.device_name.clone(),
            created_at: self.created_at.clone(),
        }
    }

    // --- v2 encryption / decryption ---

    fn encrypt_v2(
        signing_key: &SigningKey,
        verifying_key: &VerifyingKey,
        device_name: &str,
        created_at: &str,
        password: &str,
    ) -> Result<IdentityFileV2, IdentityError> {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let key = Self::derive_aead_key(&salt, password);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| IdentityError::AeadFailure(e.to_string()))?;
        let aad = Self::aead_aad();
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: signing_key.to_bytes().as_slice(),
                    aad: aad.as_slice(),
                },
            )
            .map_err(|e| IdentityError::AeadFailure(e.to_string()))?;

        Ok(IdentityFileV2 {
            format_version: FORMAT_VERSION,
            identity_scheme: IDENTITY_SCHEME.to_string(),
            michi_id: MichiId::from_public_key(verifying_key).to_base64url(),
            public_key: IdentityManager::b64_standard(verifying_key.to_bytes().as_slice()),
            encrypted_secret: IdentityManager::b64_standard(&ciphertext),
            salt: IdentityManager::b64_standard(&salt),
            nonce: IdentityManager::b64_standard(&nonce),
            created_at: created_at.to_string(),
            device_name: device_name.to_string(),
        })
    }

    fn decrypt_v2(file: &IdentityFileV2, password: &str) -> Result<Self, IdentityError> {
        if file.format_version != FORMAT_VERSION {
            return Err(IdentityError::IdentityCorrupted(format!(
                "unsupported format_version {}",
                file.format_version
            )));
        }
        if file.identity_scheme != IDENTITY_SCHEME {
            return Err(IdentityError::IdentityCorrupted(format!(
                "unsupported identity_scheme {}",
                file.identity_scheme
            )));
        }

        let salt = Self::decode_b64(&file.salt)?;
        let salt_arr: [u8; 16] = salt
            .try_into()
            .map_err(|_| IdentityError::IdentityCorrupted("invalid salt length".into()))?;
        let nonce = Self::decode_b64(&file.nonce)?;
        let ciphertext = Self::decode_b64(&file.encrypted_secret)?;

        let key = Self::derive_aead_key(&salt_arr, password);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| IdentityError::AeadFailure(e.to_string()))?;
        let aad = Self::aead_aad();
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: ciphertext.as_slice(),
                    aad: aad.as_slice(),
                },
            )
            .map_err(|_| IdentityError::InvalidPassword)?;

        let secret: [u8; 32] = plaintext
            .try_into()
            .map_err(|_| IdentityError::IdentityCorrupted("invalid secret key length".into()))?;
        let signer = SigningKey::from_bytes(&secret);
        let verifying_key = signer.verifying_key();

        Self::finalize_load(
            file.device_name.clone(),
            file.created_at.clone(),
            signer,
            verifying_key,
            &file.michi_id,
        )
    }

    // --- Legacy v1 migration ---

    fn migrate_v1_to_v2(
        legacy: &IdentityFileV1,
        file_path: &Path,
        password: &str,
    ) -> Result<Self, IdentityError> {
        let secret_key = Self::legacy_unwrap_key(&legacy.wrapped_secret_key, &legacy.salt)?;
        let secret: [u8; 32] = secret_key
            .try_into()
            .map_err(|_| IdentityError::MigrationFailed("invalid secret key length".into()))?;

        let signer = SigningKey::from_bytes(&secret);
        let verifying_key = signer.verifying_key();

        // Verify the migrated key corresponds to the stored public key.
        if verifying_key.to_bytes().as_slice() != legacy.public_key.as_slice() {
            return Err(IdentityError::MigrationFailed(
                "secret key does not match the stored public key".into(),
            ));
        }
        // Verify the stored michi_id is consistent.
        let computed = blake3::hash(&verifying_key.to_bytes());
        if computed.as_bytes() != &legacy.michi_id {
            return Err(IdentityError::MigrationFailed(
                "stored michi_id does not match the secret key".into(),
            ));
        }

        // Re-encrypt in the v2 format, preserving created_at and device_name.
        let file = Self::encrypt_v2(
            &signer,
            &verifying_key,
            &legacy.device_name,
            &legacy.created_at,
            password,
        )?;
        // Atomic replace also invalidates the legacy format on disk.
        Self::write_file(file_path, &file)?;

        tracing::info!("Identity migrated from legacy format to v2");
        Self::finalize_load(
            legacy.device_name.clone(),
            legacy.created_at.clone(),
            signer,
            verifying_key,
            &file.michi_id,
        )
    }

    fn legacy_unwrap_key(wrapped: &[u8], salt: &[u8; 16]) -> Result<Vec<u8>, IdentityError> {
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        let mut hasher = blake3::Hasher::new();
        hasher.update(LEGACY_WRAP_CONTEXT);
        hasher.update(hostname.as_bytes());
        hasher.update(salt);
        let wrap_key = *hasher.finalize().as_bytes();
        Ok(wrapped
            .iter()
            .zip(wrap_key.iter().cycle())
            .map(|(w, k)| w ^ k)
            .collect())
    }

    /// Shared tail of every load path: integrity checks + struct assembly.
    fn finalize_load(
        device_name: String,
        created_at: String,
        signer: SigningKey,
        verifying_key: VerifyingKey,
        stored_michi_id_b64: &str,
    ) -> Result<Self, IdentityError> {
        // Cross-check the stored michi_id against the derived identity.
        let computed = MichiId::from_public_key(&verifying_key);
        if computed.to_base64url() != stored_michi_id_b64 {
            return Err(IdentityError::KeyMismatch(
                "stored michi_id does not match the loaded keypair".into(),
            ));
        }
        let michi_id = MichiId::from_base64url(stored_michi_id_b64)
            .ok_or_else(|| IdentityError::KeyMismatch("invalid stored michi_id".into()))?;

        tracing::info!("Identity loaded: michi_id={}", michi_id);
        Ok(Self {
            signer,
            verifying_key,
            michi_id,
            device_name,
            created_at,
        })
    }

    // --- Helpers ---

    fn derive_aead_key(salt: &[u8; 16], password: &str) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(FILE_CONTEXT);
        hasher.update(salt);
        hasher.update(password.as_bytes());
        *hasher.finalize().as_bytes()
    }

    fn aead_aad() -> Vec<u8> {
        let mut aad = FILE_CONTEXT.to_vec();
        aad.extend_from_slice(IDENTITY_SCHEME.as_bytes());
        aad
    }

    fn b64_standard(data: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(data)
    }

    fn decode_b64(data: &str) -> Result<Vec<u8>, IdentityError> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data))
            .map_err(|e| IdentityError::Internal(format!("invalid base64: {}", e)))
    }

    /// Atomic write: temp file in the same directory, sync, 0600, rename.
    fn write_file(path: &Path, file: &IdentityFileV2) -> Result<(), IdentityError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = rmp_serde::to_vec(file)?;

        let tmp = path.with_file_name(format!(
            "{}.tmp.{}",
            path.file_name()
                .ok_or_else(|| IdentityError::KeySaveFailed("invalid path".into()))?
                .to_string_lossy(),
            std::process::id()
        ));

        std::fs::write(&tmp, &data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        // Flush to disk before rename so a crash never leaves a truncated target.
        let f = std::fs::File::open(&tmp)?;
        f.sync_all()?;

        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(IdentityError::KeySaveFailed(e.to_string()));
        }
        Ok(())
    }

    #[cfg(test)]
    fn new_in_memory(
        signer: SigningKey,
        verifying_key: VerifyingKey,
        michi_id: MichiId,
        device_name: &str,
    ) -> Self {
        Self {
            signer,
            verifying_key,
            michi_id,
            device_name: device_name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use tempfile::TempDir;

    const PASSWORD: &str = "test-password-123";

    fn gen(dir: &Path) -> IdentityManager {
        IdentityManager::generate(dir, "test-device", PASSWORD).unwrap()
    }

    #[test]
    fn test_create_and_reload() {
        let dir = TempDir::new().unwrap();
        let mgr = gen(dir.path());
        assert!(mgr.michi_id().is_present());
        assert_eq!(mgr.device_name(), "test-device");
        assert_eq!(mgr.michi_id().to_base64url().len(), 43);

        let loaded = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(mgr.michi_id(), loaded.michi_id());
        assert_eq!(mgr.created_at(), loaded.created_at());
    }

    #[test]
    fn test_persistence_survives_reload() {
        let dir = TempDir::new().unwrap();
        let mgr = gen(dir.path());
        let payload = b"persist-me";
        let (sig, pk) = mgr.sign_standard(payload);

        let loaded = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert!(IdentityManager::verify(payload, &sig, &pk).unwrap());
        assert_eq!(loaded.public_key_b64(), mgr.public_key_b64());
    }

    #[test]
    fn test_wrong_password() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let err = IdentityManager::load(dir.path(), "wrong-password").unwrap_err();
        assert!(
            matches!(err, IdentityError::InvalidPassword),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_tampered_ciphertext_fails_without_regen() {
        let dir = TempDir::new().unwrap();
        let mgr = gen(dir.path());
        let original = mgr.michi_id();

        let path = dir.path().join(IDENTITY_FILE);
        let data = std::fs::read(&path).unwrap();
        let mut file: IdentityFileV2 = rmp_serde::from_slice(&data).unwrap();
        // Flip one byte of the encrypted secret.
        let ct = base64::engine::general_purpose::STANDARD
            .decode(&file.encrypted_secret)
            .unwrap();
        let mut mutated = ct.clone();
        mutated[0] ^= 0x01;
        file.encrypted_secret = base64::engine::general_purpose::STANDARD.encode(&mutated);
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();

        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::InvalidPassword),
            "got {:?}",
            err
        );

        // Identity is NOT regenerated: the file still fails to load.
        let still = IdentityManager::load(dir.path(), PASSWORD);
        assert!(still.is_err());
        // And the manager object still holds the original identity.
        assert_eq!(mgr.michi_id(), original);
    }

    #[test]
    fn test_tampered_nonce_fails() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let data = std::fs::read(&path).unwrap();
        let mut file: IdentityFileV2 = rmp_serde::from_slice(&data).unwrap();
        let mut nonce = base64::engine::general_purpose::STANDARD
            .decode(&file.nonce)
            .unwrap();
        nonce[0] ^= 0x01;
        file.nonce = base64::engine::general_purpose::STANDARD.encode(&nonce);
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();

        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::InvalidPassword),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_wrong_aad_fails() {
        let dir = TempDir::new().unwrap();
        // Encrypt with a different AAD context manually, then try to load.
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let mut hasher = blake3::Hasher::new();
        hasher.update(FILE_CONTEXT);
        hasher.update(&salt);
        hasher.update(PASSWORD.as_bytes());
        let key = *hasher.finalize().as_bytes();
        let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
        // Deliberately WRONG AAD.
        let wrong_aad = b"michi-identity-file-v1-wrong";
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: signing_key.to_bytes().as_slice(),
                    aad: wrong_aad.as_slice(),
                },
            )
            .unwrap();

        let file = IdentityFileV2 {
            format_version: FORMAT_VERSION,
            identity_scheme: IDENTITY_SCHEME.to_string(),
            michi_id: MichiId::from_public_key(&verifying_key).to_base64url(),
            public_key: IdentityManager::b64_standard(verifying_key.to_bytes().as_slice()),
            encrypted_secret: IdentityManager::b64_standard(&ciphertext),
            salt: IdentityManager::b64_standard(&salt),
            nonce: IdentityManager::b64_standard(&nonce),
            created_at: chrono::Utc::now().to_rfc3339(),
            device_name: "aad-test".into(),
        };
        let path = dir.path().join(IDENTITY_FILE);
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();

        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::InvalidPassword),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_truncated_file_fails() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let data = std::fs::read(&path).unwrap();
        std::fs::write(&path, &data[..data.len() / 2]).unwrap();

        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::IdentityCorrupted(_))
                || matches!(err, IdentityError::InvalidPassword),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_michi_id_inconsistent_fails() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let data = std::fs::read(&path).unwrap();
        let mut file: IdentityFileV2 = rmp_serde::from_slice(&data).unwrap();
        // Replace stored michi_id with an unrelated valid value.
        file.michi_id =
            MichiId::from_public_key(&VerifyingKey::from_bytes(&[1u8; 32]).unwrap()).to_base64url();
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();

        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::KeyMismatch(_)),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_created_at_preserved() {
        let dir = TempDir::new().unwrap();
        let mgr = gen(dir.path());
        let loaded = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(mgr.created_at(), loaded.created_at());
    }

    #[test]
    fn test_load_or_generate_idempotent() {
        let dir = TempDir::new().unwrap();
        let mgr = IdentityManager::load_or_generate(dir.path(), "test", PASSWORD).unwrap();
        let mgr2 = IdentityManager::load_or_generate(dir.path(), "test", PASSWORD).unwrap();
        assert_eq!(mgr.michi_id(), mgr2.michi_id());
    }

    #[test]
    fn test_load_or_generate_does_not_heal_corruption() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        std::fs::write(&path, b"garbage").unwrap();

        // Corruption is NOT silently healed: load_or_generate returns the error.
        let err = IdentityManager::load_or_generate(dir.path(), "test", PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::IdentityCorrupted(_))
                || matches!(err, IdentityError::Internal(_)),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_migration_from_v1() {
        use base64::Engine;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(IDENTITY_FILE);

        // Build a legacy v1 file with the XOR scheme.
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        let mut hasher = blake3::Hasher::new();
        hasher.update(LEGACY_WRAP_CONTEXT);
        hasher.update(hostname.as_bytes());
        hasher.update(&salt);
        let wrap_key = *hasher.finalize().as_bytes();
        let wrapped: Vec<u8> = signing_key
            .to_bytes()
            .iter()
            .zip(wrap_key.iter().cycle())
            .map(|(k, w)| k ^ w)
            .collect();

        let legacy = IdentityFileV1 {
            version: 1,
            algorithm: "ed25519".into(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            device_name: "legacy-device".into(),
            wrapped_secret_key: wrapped,
            public_key: verifying_key.to_bytes().to_vec(),
            michi_id: *blake3::hash(&verifying_key.to_bytes()).as_bytes(),
            salt,
        };
        std::fs::write(&path, rmp_serde::to_vec(&legacy).unwrap()).unwrap();

        // Migrate: same identity, created_at preserved, v2 on disk.
        let migrated = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(migrated.device_name(), "legacy-device");
        assert_eq!(migrated.created_at(), "2026-01-01T00:00:00Z");
        assert_eq!(
            migrated.public_key_b64(),
            base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes())
        );
        assert_eq!(
            migrated.michi_id().to_base64url(),
            MichiId::from_public_key(&verifying_key).to_base64url()
        );

        // The file is now v2 and still loads (no identity was generated anew).
        let data = std::fs::read(&path).unwrap();
        let file: IdentityFileV2 = rmp_serde::from_slice(&data).unwrap();
        assert_eq!(file.format_version, 2);
        assert_eq!(file.identity_scheme, IDENTITY_SCHEME);
        let reloaded = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(reloaded.michi_id(), migrated.michi_id());
    }

    #[test]
    fn test_migration_rejects_wrong_legacy_key() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(IDENTITY_FILE);
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);

        // Legacy file whose wrapped secret does NOT match the stored public key.
        let other = SigningKey::generate(&mut OsRng);
        let stored_pk = SigningKey::generate(&mut OsRng).verifying_key();
        let wrapped = other.to_bytes().to_vec();
        let legacy = IdentityFileV1 {
            version: 1,
            algorithm: "ed25519".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            device_name: "x".into(),
            wrapped_secret_key: wrapped,
            public_key: stored_pk.to_bytes().to_vec(),
            michi_id: [0u8; 32],
            salt,
        };
        std::fs::write(&path, rmp_serde::to_vec(&legacy).unwrap()).unwrap();

        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::MigrationFailed(_)),
            "got {:?}",
            err
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_file_permissions_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "expected 0600, got {:o}", mode);
    }

    #[test]
    fn test_sign_and_verify() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        let mgr = IdentityManager::new_in_memory(sk, vk, MichiId::null(), "test");

        let payload = b"hello world";
        let (sig, pk) = mgr.sign_standard(payload);
        let valid = IdentityManager::verify(payload, &sig, &pk).unwrap();
        assert!(valid);

        let invalid = IdentityManager::verify(b"wrong", &sig, &pk).unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_derive_michi_id_matches() {
        let mgr = gen(TempDir::new().unwrap().path());
        let derived = IdentityManager::derive_michi_id(&mgr.public_key_b64()).unwrap();
        assert_eq!(derived, *mgr.michi_id());
    }
}
