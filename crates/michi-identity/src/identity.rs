//! IdentityManager: generation, persistence and operations with Ed25519 keys.
//!
//! ## Cryptographic flow (contract v1)
//!
//! ```text
//! OS RNG ──→ ed25519-dalek ──→ Keypair
//!                                 │
//!                                 ├── secret_key → ChaCha20-Poly1305 (AEAD)
//!                                 │      key  = Argon2id(password, salt)
//!                                 │             (64 MiB, t=3, p=1, 0x13)
//!                                 │      AAD  = canonical IdentityHeader
//!                                 │             (all persisted metadata)
//!                                 │      └── identity.msgpack (format_version 3)
//!                                 │
//!                                 └── public_key ──→ blake3 ──→ michi_id
//!                                                          (base64url, 43 chars)
//! ```
//!
//! ## Guarantees
//!
//! - At-rest encryption is authenticated (ChaCha20-Poly1305) with an AAD that
//!   binds the ciphertext to every persisted metadata field: mutating any
//!   header field breaks authentication.
//! - The KDF is Argon2id (64 MiB, 3 iterations, 1 lane) — a memory-hard KDF,
//!   not a fast hash. Parameters are persisted and validated on load.
//! - Writes are atomic (temp file + rename) with `0600` permissions on Unix.
//! - A corrupted or unauthenticated file returns an explicit error and NEVER
//!   silently regenerates the identity.
//! - Legacy formats migrate automatically: v1 (XOR obfuscation) and v2
//!   (BLAKE3-direct KDF) → v3 (Argon2id). Identity and `created_at` are kept.
//!
//! ## Error separation
//!
//! - `UnsupportedFormat`: unknown format version / scheme / KDF name.
//! - `MetadataTampered`: header fields structurally invalid (KDF params,
//!   lengths) — detectable before decryption.
//! - `AuthenticationFailed`: AEAD authentication failure. Deliberately covers
//!   BOTH wrong password and tampering of authenticated fields: exposing the
//!   difference would create an oracle.
//! - `IdentityMismatch`: post-decryption key/identity incoherence (defense in
//!   depth; cannot normally be reached with AAD-bound metadata).
//!
//! ## Recovery procedure
//!
//! If the identity file is corrupted, the operator must restore it from a
//! backup. Identity rotation is an explicit, user-initiated action:
//! delete the file and call `load_or_generate` — the library never does this
//! implicitly.

use std::path::Path;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::error::IdentityError;
use crate::types::{decode_base64url_strict, encode_base64url, MichiId};

const IDENTITY_FILE: &str = "identity.msgpack";
const FORMAT_VERSION: u32 = 3;
const IDENTITY_SCHEME: &str = "ed25519-blake3-v1";

// --- Argon2id parameters (contract v1 storage) ---
const KDF_NAME: &str = "argon2id";
const KDF_VERSION: u32 = 0x13;
const KDF_MEMORY_KIB: u32 = 64 * 1024;
const KDF_ITERATIONS: u32 = 3;
const KDF_PARALLELISM: u32 = 1;
const KDF_OUTPUT_LEN: usize = 32;
const MIN_MEMORY_KIB: u32 = 8 * 1024;

/// Context binding the v2 (legacy) BLAKE3 KDF — migration only.
const V2_FILE_CONTEXT: &[u8] = b"michi-identity-file-v2";
/// Legacy (v1) wrap key context — migration only.
const LEGACY_WRAP_CONTEXT: &[u8] = b"michi-identity-key-wrap-v1";

/// Identity file persisted on disk (MessagePack, format v3).
#[derive(Debug, Serialize, Deserialize)]
struct IdentityFileV3 {
    format_version: u32,
    identity_scheme: String,
    michi_id: String,
    public_key: String,
    encrypted_secret: String,
    kdf: String,
    kdf_version: u32,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    salt: String,
    nonce: String,
    created_at: String,
    device_name: String,
}

/// Canonical header: every persisted metadata field, used in full as AAD.
#[derive(Debug, Serialize, Deserialize)]
struct IdentityHeader {
    format_version: u32,
    identity_scheme: String,
    michi_id: String,
    public_key: String,
    created_at: String,
    device_name: String,
    kdf: String,
    kdf_version: u32,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    salt: String,
}

/// Legacy v2 file (BLAKE3-direct KDF) — accepted only for migration.
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

/// Legacy v1 file (XOR obfuscation) — accepted only for migration.
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
    /// Generates a new keypair and persists it (atomic, Argon2id + AEAD).
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
        let file = Self::encrypt_v3(
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
    /// Migrates legacy formats (v1 XOR, v2 BLAKE3-KDF) automatically. Returns
    /// an explicit error on corruption, wrong password or tampering — it never
    /// regenerates.
    pub fn load(config_dir: &Path, password: &str) -> Result<Self, IdentityError> {
        let file_path = config_dir.join(IDENTITY_FILE);
        if !file_path.exists() {
            return Err(IdentityError::KeyNotFound(file_path.display().to_string()));
        }
        let data = std::fs::read(&file_path)?;
        Self::load_bytes(&data, &file_path, password)
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
            return Self::load(config_dir, password);
        }
        Self::generate(config_dir, device_name, password)
    }

    fn load_bytes(data: &[u8], file_path: &Path, password: &str) -> Result<Self, IdentityError> {
        // Try canonical v3.
        if let Ok(file) = rmp_serde::from_slice::<IdentityFileV3>(data) {
            if file.format_version == FORMAT_VERSION {
                return Self::decrypt_v3(&file, password);
            }
        }
        // Legacy v2 (BLAKE3 KDF).
        if let Ok(file) = rmp_serde::from_slice::<IdentityFileV2>(data) {
            if file.format_version == 2 {
                tracing::info!("Legacy v2 identity format detected, migrating to v3");
                return Self::migrate_v2_to_v3(&file, file_path, password);
            }
        }
        // Legacy v1 (XOR).
        if let Ok(file) = rmp_serde::from_slice::<IdentityFileV1>(data) {
            if file.version == 1 {
                tracing::info!("Legacy v1 identity format detected, migrating to v3");
                return Self::migrate_v1_to_v3(&file, file_path, password);
            }
        }
        Err(IdentityError::IdentityCorrupted(format!(
            "unrecognized identity file format: {}",
            file_path.display()
        )))
    }

    /// Signs a payload and returns (signature_base64url, public_key_base64url).
    pub fn sign_base64url(&self, payload: &[u8]) -> (String, String) {
        let sig = self.signer.sign(payload);
        (
            encode_base64url(&sig.to_bytes()),
            encode_base64url(&self.verifying_key.to_bytes()),
        )
    }

    /// Signs a payload and returns (signature_bytes_raw, public_key_bytes_raw).
    pub fn sign_raw(&self, payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let sig = self.signer.sign(payload);
        (
            sig.to_bytes().to_vec(),
            self.verifying_key.to_bytes().to_vec(),
        )
    }

    /// Verifies an Ed25519 signature. Accepts base64url (canonical) and, for
    /// legacy callers only, base64 STANDARD.
    pub fn verify(
        payload: &[u8],
        signature_b64: &str,
        public_key_b64: &str,
    ) -> Result<bool, IdentityError> {
        let sig_bytes = Self::decode_wire_b64(signature_b64)?;
        let pk_bytes = Self::decode_wire_b64(public_key_b64)?;

        let sig = Signature::from_slice(&sig_bytes)?;
        let pk =
            VerifyingKey::from_bytes(&pk_bytes.try_into().map_err(|_| {
                IdentityError::InvalidSignature("public key must be 32 bytes".into())
            })?)?;

        Ok(pk.verify_strict(payload, &sig).is_ok())
    }

    /// Derives a MichiId from a base64url public key (accepts STANDARD too
    /// for legacy callers).
    pub fn derive_michi_id(public_key_b64: &str) -> Result<MichiId, IdentityError> {
        let pk_bytes = Self::decode_wire_b64(public_key_b64)?;
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

    /// Returns the public key in canonical base64url (43 chars, no padding).
    pub fn public_key_base64url(&self) -> String {
        encode_base64url(&self.verifying_key.to_bytes())
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
            public_key: self.public_key_base64url(),
            identity_scheme: IDENTITY_SCHEME.to_string(),
            device_name: self.device_name.clone(),
            created_at: self.created_at.clone(),
        }
    }

    // --- v3 encryption / decryption (Argon2id + ChaCha20-Poly1305) ---

    fn encrypt_v3(
        signing_key: &SigningKey,
        verifying_key: &VerifyingKey,
        device_name: &str,
        created_at: &str,
        password: &str,
    ) -> Result<IdentityFileV3, IdentityError> {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let file = IdentityFileV3 {
            format_version: FORMAT_VERSION,
            identity_scheme: IDENTITY_SCHEME.to_string(),
            michi_id: MichiId::from_public_key(verifying_key).to_base64url(),
            public_key: encode_base64url(&verifying_key.to_bytes()),
            encrypted_secret: String::new(), // filled after encryption
            kdf: KDF_NAME.to_string(),
            kdf_version: KDF_VERSION,
            memory_kib: KDF_MEMORY_KIB,
            iterations: KDF_ITERATIONS,
            parallelism: KDF_PARALLELISM,
            salt: encode_base64url(&salt),
            nonce: encode_base64url(&nonce),
            created_at: created_at.to_string(),
            device_name: device_name.to_string(),
        };

        let ciphertext = Self::aead_encrypt(
            &file,
            signing_key.to_bytes().as_slice(),
            &salt,
            &nonce,
            password,
        )?;
        Ok(IdentityFileV3 {
            encrypted_secret: encode_base64url(&ciphertext),
            ..file
        })
    }

    fn decrypt_v3(file: &IdentityFileV3, password: &str) -> Result<Self, IdentityError> {
        Self::validate_header_v3(file)?;
        let salt = decode_base64url_strict(&file.salt)
            .map_err(|_| IdentityError::MetadataTampered("invalid salt encoding".into()))?;
        let nonce = decode_base64url_strict(&file.nonce)
            .map_err(|_| IdentityError::MetadataTampered("invalid nonce encoding".into()))?;
        let ciphertext = decode_base64url_strict(&file.encrypted_secret)
            .map_err(|_| IdentityError::MetadataTampered("invalid ciphertext encoding".into()))?;

        let aad = Self::canonical_header_bytes(file)?;
        let plaintext = Self::aead_decrypt(&aad, &ciphertext, &salt, &nonce, password)?;

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

    /// Structural validation of the v3 header BEFORE any key derivation.
    fn validate_header_v3(file: &IdentityFileV3) -> Result<(), IdentityError> {
        if file.format_version != FORMAT_VERSION {
            return Err(IdentityError::UnsupportedFormat(format!(
                "format_version {}",
                file.format_version
            )));
        }
        if file.identity_scheme != IDENTITY_SCHEME {
            return Err(IdentityError::UnsupportedFormat(format!(
                "identity_scheme {}",
                file.identity_scheme
            )));
        }
        if file.kdf != KDF_NAME {
            return Err(IdentityError::UnsupportedFormat(format!(
                "kdf {}",
                file.kdf
            )));
        }
        if file.kdf_version != KDF_VERSION {
            return Err(IdentityError::UnsupportedFormat(format!(
                "kdf_version {}",
                file.kdf_version
            )));
        }
        if file.memory_kib < MIN_MEMORY_KIB {
            return Err(IdentityError::MetadataTampered(format!(
                "memory_kib {} below minimum {}",
                file.memory_kib, MIN_MEMORY_KIB
            )));
        }
        if file.iterations == 0 || file.parallelism == 0 {
            return Err(IdentityError::MetadataTampered(
                "iterations/parallelism must be >= 1".into(),
            ));
        }
        let salt = decode_base64url_strict(&file.salt)
            .map_err(|_| IdentityError::MetadataTampered("invalid salt".into()))?;
        if salt.len() != 16 {
            return Err(IdentityError::MetadataTampered(
                "salt must be 16 bytes".into(),
            ));
        }
        Ok(())
    }

    /// Deterministic serialization of the full metadata header (used as AAD).
    fn canonical_header_bytes(file: &IdentityFileV3) -> Result<Vec<u8>, IdentityError> {
        let header = IdentityHeader {
            format_version: file.format_version,
            identity_scheme: file.identity_scheme.clone(),
            michi_id: file.michi_id.clone(),
            public_key: file.public_key.clone(),
            created_at: file.created_at.clone(),
            device_name: file.device_name.clone(),
            kdf: file.kdf.clone(),
            kdf_version: file.kdf_version,
            memory_kib: file.memory_kib,
            iterations: file.iterations,
            parallelism: file.parallelism,
            salt: file.salt.clone(),
        };
        rmp_serde::to_vec(&header)
            .map_err(|e| IdentityError::IdentityCorrupted(format!("header serialization: {}", e)))
    }

    fn kdf_argon2id(password: &str, salt: &[u8]) -> Result<[u8; KDF_OUTPUT_LEN], IdentityError> {
        let params = Params::new(
            KDF_MEMORY_KIB,
            KDF_ITERATIONS,
            KDF_PARALLELISM,
            Some(KDF_OUTPUT_LEN),
        )
        .map_err(|e| IdentityError::Internal(format!("argon2 params: {}", e)))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut out = [0u8; KDF_OUTPUT_LEN];
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut out)
            .map_err(|e| IdentityError::Internal(format!("argon2: {}", e)))?;
        Ok(out)
    }

    fn aead_encrypt(
        file: &IdentityFileV3,
        secret: &[u8],
        salt: &[u8; 16],
        nonce: &[u8; 12],
        password: &str,
    ) -> Result<Vec<u8>, IdentityError> {
        let key = Self::kdf_argon2id(password, salt)?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| IdentityError::aead_failure(e.to_string()))?;
        let aad = Self::canonical_header_bytes(file)?;
        cipher
            .encrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: secret,
                    aad: aad.as_slice(),
                },
            )
            .map_err(|e| IdentityError::aead_failure(e.to_string()))
    }

    fn aead_decrypt(
        aad: &[u8],
        ciphertext: &[u8],
        salt: &[u8],
        nonce: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, IdentityError> {
        let salt_arr: [u8; 16] = salt
            .try_into()
            .map_err(|_| IdentityError::MetadataTampered("invalid salt length".into()))?;
        let key = Self::kdf_argon2id(password, &salt_arr)?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| IdentityError::aead_failure(e.to_string()))?;
        cipher
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ciphertext,
                    aad,
                },
            )
            .map_err(|_| IdentityError::AuthenticationFailed)
    }

    // --- Legacy migrations ---

    /// v2 (BLAKE3-direct KDF) → v3 (Argon2id). Identity and created_at kept.
    fn migrate_v2_to_v3(
        legacy: &IdentityFileV2,
        file_path: &Path,
        password: &str,
    ) -> Result<Self, IdentityError> {
        let salt = decode_base64url_strict(&legacy.salt)
            .map_err(|_| IdentityError::MigrationFailed("invalid v2 salt".into()))?;
        let salt_arr: [u8; 16] = salt
            .try_into()
            .map_err(|_| IdentityError::MigrationFailed("invalid v2 salt length".into()))?;
        let nonce = decode_base64url_strict(&legacy.nonce)
            .map_err(|_| IdentityError::MigrationFailed("invalid v2 nonce".into()))?;
        let ciphertext = decode_base64url_strict(&legacy.encrypted_secret)
            .map_err(|_| IdentityError::MigrationFailed("invalid v2 ciphertext".into()))?;

        // v2 key derivation: blake3(V2_FILE_CONTEXT || salt || password).
        let mut hasher = blake3::Hasher::new();
        hasher.update(V2_FILE_CONTEXT);
        hasher.update(&salt_arr);
        hasher.update(password.as_bytes());
        let key = *hasher.finalize().as_bytes();

        let mut aad = V2_FILE_CONTEXT.to_vec();
        aad.extend_from_slice(legacy.identity_scheme.as_bytes());

        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| IdentityError::aead_failure(e.to_string()))?;
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: ciphertext.as_slice(),
                    aad: aad.as_slice(),
                },
            )
            .map_err(|_| IdentityError::AuthenticationFailed)?;

        let secret: [u8; 32] = plaintext
            .try_into()
            .map_err(|_| IdentityError::MigrationFailed("invalid secret length".into()))?;
        let signer = SigningKey::from_bytes(&secret);
        let verifying_key = signer.verifying_key();

        // Verify the migrated key against the stored identity.
        if encode_base64url(&verifying_key.to_bytes()) != legacy.public_key {
            return Err(IdentityError::MigrationFailed(
                "secret key does not match the stored public key".into(),
            ));
        }
        let computed = MichiId::from_public_key(&verifying_key);
        if computed.to_base64url() != legacy.michi_id {
            return Err(IdentityError::MigrationFailed(
                "stored michi_id does not match the secret key".into(),
            ));
        }

        // Re-encrypt in v3, preserving created_at and device_name.
        let file = Self::encrypt_v3(
            &signer,
            &verifying_key,
            &legacy.device_name,
            &legacy.created_at,
            password,
        )?;
        Self::write_file(file_path, &file)?;

        tracing::info!("Identity migrated v2 -> v3");
        Self::decrypt_v3(&file, password)
    }

    /// v1 (XOR obfuscation) → v3 (Argon2id). Identity and created_at kept.
    fn migrate_v1_to_v3(
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
        let computed = blake3::hash(&verifying_key.to_bytes());
        if computed.as_bytes() != &legacy.michi_id {
            return Err(IdentityError::MigrationFailed(
                "stored michi_id does not match the secret key".into(),
            ));
        }

        let file = Self::encrypt_v3(
            &signer,
            &verifying_key,
            &legacy.device_name,
            &legacy.created_at,
            password,
        )?;
        Self::write_file(file_path, &file)?;

        tracing::info!("Identity migrated v1 -> v3");
        Self::decrypt_v3(&file, password)
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
        // Defense in depth: the AAD already authenticates these fields, but
        // double-check the keypair is coherent with the stored identity.
        let computed = MichiId::from_public_key(&verifying_key);
        if computed.to_base64url() != stored_michi_id_b64 {
            return Err(IdentityError::IdentityMismatch(
                "stored michi_id does not match the loaded keypair".into(),
            ));
        }
        let michi_id = MichiId::from_base64url(stored_michi_id_b64)
            .ok_or_else(|| IdentityError::IdentityMismatch("invalid stored michi_id".into()))?;

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

    fn decode_wire_b64(data: &str) -> Result<Vec<u8>, IdentityError> {
        decode_base64url_strict(data).or_else(|_| Self::legacy_decode_standard(data))
    }

    /// Legacy tolerance: base64 STANDARD (pre-v1 contract). Never emitted.
    fn legacy_decode_standard(data: &str) -> Result<Vec<u8>, IdentityError> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| IdentityError::InvalidSignature(format!("invalid base64: {}", e)))
    }

    /// Atomic write: temp file in the same directory, sync, 0600, rename.
    fn write_file(path: &Path, file: &IdentityFileV3) -> Result<(), IdentityError> {
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

// Small local helper to keep AEAD map_err readable.
impl IdentityError {
    fn aead_failure(msg: String) -> Self {
        IdentityError::Internal(format!("aead: {}", msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const PASSWORD: &str = "test-password-123";

    fn gen(dir: &Path) -> IdentityManager {
        IdentityManager::generate(dir, "test-device", PASSWORD).unwrap()
    }

    fn read_v3(path: &Path) -> IdentityFileV3 {
        let data = std::fs::read(path).unwrap();
        rmp_serde::from_slice(&data).unwrap()
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
        let (sig, pk) = mgr.sign_base64url(payload);

        let loaded = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert!(IdentityManager::verify(payload, &sig, &pk).unwrap());
        assert_eq!(loaded.public_key_base64url(), mgr.public_key_base64url());
    }

    #[test]
    fn test_wire_values_are_base64url() {
        let dir = TempDir::new().unwrap();
        let mgr = gen(dir.path());
        let pk = mgr.public_key_base64url();
        assert_eq!(pk.len(), 43);
        assert!(!pk.contains('+') && !pk.contains('/') && !pk.contains('='));
        let (sig, _) = mgr.sign_base64url(b"hello");
        assert_eq!(sig.len(), 86);
        assert!(!sig.contains('+') && !sig.contains('/') && !sig.contains('='));
    }

    #[test]
    fn test_argon2_parameters_persisted() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let file = read_v3(&dir.path().join(IDENTITY_FILE));
        assert_eq!(file.format_version, 3);
        assert_eq!(file.kdf, "argon2id");
        assert_eq!(file.kdf_version, 0x13);
        assert_eq!(file.memory_kib, 64 * 1024);
        assert_eq!(file.iterations, 3);
        assert_eq!(file.parallelism, 1);
    }

    #[test]
    fn test_wrong_password_v3() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let err = IdentityManager::load(dir.path(), "wrong-password").unwrap_err();
        assert!(
            matches!(err, IdentityError::AuthenticationFailed),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_tampered_kdf_parameters_rejected() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mut file = read_v3(&path);
        file.memory_kib = 1024; // below minimum
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();
        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::MetadataTampered(_)),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_tampered_kdf_name_rejected() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mut file = read_v3(&path);
        file.kdf = "fast-hash".into();
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();
        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::UnsupportedFormat(_)),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_tampered_device_name_rejected() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mut file = read_v3(&path);
        file.device_name = "EVIL".into();
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();
        // device_name is part of the AAD: tampering breaks authentication.
        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::AuthenticationFailed),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_tampered_created_at_rejected() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mut file = read_v3(&path);
        file.created_at = "1970-01-01T00:00:00Z".into();
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();
        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::AuthenticationFailed),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_tampered_public_key_rejected() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mut file = read_v3(&path);
        file.public_key = "A".repeat(43);
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();
        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::AuthenticationFailed),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_tampered_michi_id_rejected() {
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mut file = read_v3(&path);
        file.michi_id = "B".repeat(43);
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();
        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::AuthenticationFailed),
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
        let mut file = read_v3(&path);
        let mut ct = decode_base64url_strict(&file.encrypted_secret).unwrap();
        ct[0] ^= 0x01;
        file.encrypted_secret = encode_base64url(&ct);
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();

        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::AuthenticationFailed),
            "got {:?}",
            err
        );
        assert_eq!(mgr.michi_id(), original);
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
                || matches!(err, IdentityError::Internal(_)),
            "got {:?}",
            err
        );
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

        let err = IdentityManager::load_or_generate(dir.path(), "test", PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::IdentityCorrupted(_))
                || matches!(err, IdentityError::Internal(_)),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_v2_to_v3_migration() {
        use crate::types::encode_base64url;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(IDENTITY_FILE);

        // Build a legacy v2 file (BLAKE3-direct KDF, v2 AAD).
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let mut hasher = blake3::Hasher::new();
        hasher.update(V2_FILE_CONTEXT);
        hasher.update(&salt);
        hasher.update(PASSWORD.as_bytes());
        let key = *hasher.finalize().as_bytes();
        let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
        let mut aad = V2_FILE_CONTEXT.to_vec();
        aad.extend_from_slice(IDENTITY_SCHEME.as_bytes());
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: signing_key.to_bytes().as_slice(),
                    aad: aad.as_slice(),
                },
            )
            .unwrap();

        let legacy = IdentityFileV2 {
            format_version: 2,
            identity_scheme: IDENTITY_SCHEME.to_string(),
            michi_id: MichiId::from_public_key(&verifying_key).to_base64url(),
            public_key: encode_base64url(&verifying_key.to_bytes()),
            encrypted_secret: encode_base64url(&ciphertext),
            salt: encode_base64url(&salt),
            nonce: encode_base64url(&nonce),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            device_name: "legacy-v2-device".into(),
        };
        std::fs::write(&path, rmp_serde::to_vec(&legacy).unwrap()).unwrap();

        let migrated = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(migrated.device_name(), "legacy-v2-device");
        assert_eq!(migrated.created_at(), "2026-01-01T00:00:00Z");
        assert_eq!(
            migrated.public_key_base64url(),
            encode_base64url(&verifying_key.to_bytes())
        );

        // File is now v3 with Argon2id and still loads.
        let file = read_v3(&path);
        assert_eq!(file.format_version, 3);
        assert_eq!(file.kdf, "argon2id");
        let reloaded = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(reloaded.michi_id(), migrated.michi_id());
    }

    #[test]
    fn test_v1_to_v3_migration() {
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
            device_name: "legacy-v1-device".into(),
            wrapped_secret_key: wrapped,
            public_key: verifying_key.to_bytes().to_vec(),
            michi_id: *blake3::hash(&verifying_key.to_bytes()).as_bytes(),
            salt,
        };
        std::fs::write(&path, rmp_serde::to_vec(&legacy).unwrap()).unwrap();

        let migrated = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(migrated.device_name(), "legacy-v1-device");
        assert_eq!(migrated.created_at(), "2026-01-01T00:00:00Z");
        assert_eq!(
            migrated.public_key_base64url(),
            encode_base64url(&verifying_key.to_bytes())
        );

        let file = read_v3(&path);
        assert_eq!(file.format_version, 3);
        let reloaded = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(reloaded.michi_id(), migrated.michi_id());
    }

    #[test]
    fn test_identity_preserved_after_migration() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(IDENTITY_FILE);
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let mut hasher = blake3::Hasher::new();
        hasher.update(V2_FILE_CONTEXT);
        hasher.update(&salt);
        hasher.update(PASSWORD.as_bytes());
        let key = *hasher.finalize().as_bytes();
        let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
        let mut aad = V2_FILE_CONTEXT.to_vec();
        aad.extend_from_slice(IDENTITY_SCHEME.as_bytes());
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: signing_key.to_bytes().as_slice(),
                    aad: aad.as_slice(),
                },
            )
            .unwrap();
        let expected_id = MichiId::from_public_key(&verifying_key).to_base64url();
        let legacy = IdentityFileV2 {
            format_version: 2,
            identity_scheme: IDENTITY_SCHEME.to_string(),
            michi_id: expected_id.clone(),
            public_key: encode_base64url(&verifying_key.to_bytes()),
            encrypted_secret: encode_base64url(&ciphertext),
            salt: encode_base64url(&salt),
            nonce: encode_base64url(&nonce),
            created_at: "2026-06-15T10:30:00Z".to_string(),
            device_name: "kept".into(),
        };
        std::fs::write(&path, rmp_serde::to_vec(&legacy).unwrap()).unwrap();

        let migrated = IdentityManager::load(dir.path(), PASSWORD).unwrap();
        assert_eq!(migrated.michi_id().to_base64url(), expected_id);
        assert_eq!(migrated.created_at(), "2026-06-15T10:30:00Z");
    }

    #[test]
    fn test_wrong_password_distinguishable_only_where_safe() {
        // AEAD failures deliberately do not distinguish wrong password from
        // tampered authenticated metadata (no oracle). Structural tampering
        // (KDF params) IS distinguishable and must surface as MetadataTampered.
        let dir = TempDir::new().unwrap();
        gen(dir.path());
        let path = dir.path().join(IDENTITY_FILE);
        let mut file = read_v3(&path);
        file.iterations = 0;
        std::fs::write(&path, rmp_serde::to_vec(&file).unwrap()).unwrap();
        let err = IdentityManager::load(dir.path(), PASSWORD).unwrap_err();
        assert!(
            matches!(err, IdentityError::MetadataTampered(_)),
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
        let (sig, pk) = mgr.sign_base64url(payload);
        assert_eq!(sig.len(), 86);
        assert_eq!(pk.len(), 43);
        let valid = IdentityManager::verify(payload, &sig, &pk).unwrap();
        assert!(valid);

        let invalid = IdentityManager::verify(b"wrong", &sig, &pk).unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_derive_michi_id_matches() {
        let mgr = gen(TempDir::new().unwrap().path());
        let derived = IdentityManager::derive_michi_id(&mgr.public_key_base64url()).unwrap();
        assert_eq!(derived, *mgr.michi_id());
    }

    #[test]
    fn test_contract_violation_display() {
        use crate::error::ContractViolation;
        let e = IdentityError::ContractViolation(ContractViolation::InvalidRoleProfile);
        assert!(e.to_string().contains("role profile"));
    }
}
