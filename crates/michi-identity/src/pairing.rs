use crate::error::IdentityError;
use crate::types::{AuthChallenge, PairingSession};
use rand::Rng;

/// Protocolo de emparejamiento TOFU + PIN.
pub struct PairingProtocol;

impl PairingProtocol {
    /// Genera un PIN criptográfico de 6 dígitos.
    pub fn generate_pin() -> String {
        let mut rng = rand::rngs::OsRng;
        let pin: u32 = rng.gen_range(100_000..1_000_000);
        format!("{:06}", pin)
    }

    /// Calcula pin_hash = blake3(pin + nonce).
    pub fn hash_pin(pin: &str, nonce: &[u8]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(pin.as_bytes());
        hasher.update(nonce);
        *hasher.finalize().as_bytes()
    }

    /// Verifica un PIN contra su hash en tiempo constante.
    pub fn verify_pin(pin: &str, nonce: &[u8], expected_hash: &[u8; 32]) -> bool {
        use subtle::ConstantTimeEq;
        let computed = Self::hash_pin(pin, nonce);
        computed.ct_eq(expected_hash).into()
    }

    /// Lado servidor: inicia sesión de pairing.
    ///
    /// 1. Decodifica el nonce base64 a bytes RAW.
    /// 2. Verifica la firma Ed25519 sobre esos bytes RAW.
    /// 3. Genera PIN y almacena peer info.
    pub fn start_server(
        peer_public_key: &[u8],
        client_challenge: &AuthChallenge,
    ) -> Result<(PairingSession, String), IdentityError> {
        use base64::Engine;
        // Decodificar nonce a bytes RAW (fue firmado como raw bytes)
        let nonce_bytes = base64::engine::general_purpose::STANDARD.decode(&client_challenge.nonce)?;

        let valid = crate::identity::IdentityManager::verify(
            &nonce_bytes,
            &client_challenge.signature,
            &base64::engine::general_purpose::STANDARD.encode(peer_public_key),
        )?;

        if !valid {
            return Err(IdentityError::ChallengeMismatch);
        }

        let pin = Self::generate_pin();
        let server_nonce: [u8; 16] = rand::random();
        let client_nonce = base64::engine::general_purpose::STANDARD.decode(&client_challenge.nonce)?;

        let pin_hash = Self::hash_pin(&pin, &server_nonce);

        let michi_id = crate::identity::IdentityManager::derive_michi_id(
            &base64::engine::general_purpose::STANDARD.encode(peer_public_key),
        )?;

        let session = PairingSession {
            peer_michi_id: michi_id,
            peer_public_key: peer_public_key.to_vec(),
            pin: pin.clone(),
            pin_hash,
            server_nonce: server_nonce.to_vec(),
            client_nonce,
        };

        Ok((session, pin))
    }

    /// Lado servidor: confirma pairing.
    pub fn confirm_server(
        session: &PairingSession,
        pin_input: &str,
        pin_proof_sig: &str,
        peer_public_key: &[u8],
    ) -> Result<(), IdentityError> {
        use base64::Engine;
        let pin_proof = Self::hash_pin(pin_input, &session.server_nonce);
        let valid_sig = crate::identity::IdentityManager::verify(
            &pin_proof,
            pin_proof_sig,
            &base64::engine::general_purpose::STANDARD.encode(peer_public_key),
        )?;

        if !valid_sig {
            return Err(IdentityError::InvalidSignature("pin_proof signature invalid".into()));
        }

        if !Self::verify_pin(pin_input, &session.server_nonce, &session.pin_hash) {
            return Err(IdentityError::PinMismatch);
        }

        Ok(())
    }

    /// Lado cliente: crea un challenge firmado.
    ///
    /// El nonce se firma como bytes RAW, no como string base64.
    /// El servidor verifica la firma sobre los mismos bytes RAW.
    pub fn create_challenge(
        identity: &crate::identity::IdentityManager,
    ) -> Result<AuthChallenge, IdentityError> {
        use base64::Engine;
        let nonce: [u8; 16] = rand::random();
        // Firmamos los bytes RAW del nonce
        let (sig, _) = identity.sign_standard(&nonce);
        let nonce_b64 = base64::engine::general_purpose::STANDARD.encode(nonce);

        Ok(AuthChallenge {
            michi_id: *identity.michi_id(),
            nonce: nonce_b64,
            signature: sig,
        })
    }

    /// Lado cliente: verifica el challenge del servidor.
    ///
    /// Decodifica el nonce de base64 a bytes RAW y verifica la firma
    /// contra esos bytes RAW (que es lo que firmó el servidor).
    pub fn verify_server_challenge(
        server_public_key: &[u8],
        challenge: &AuthChallenge,
    ) -> Result<(), IdentityError> {
        use base64::Engine;
        // Decodificar nonce de base64 a bytes RAW
        let nonce_bytes = base64::engine::general_purpose::STANDARD
            .decode(&challenge.nonce)?;
        let valid = crate::identity::IdentityManager::verify(
            &nonce_bytes,
            &challenge.signature,
            &base64::engine::general_purpose::STANDARD.encode(server_public_key),
        )?;

        if !valid {
            return Err(IdentityError::ChallengeMismatch);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityManager;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_generate_pin() {
        let pin1 = PairingProtocol::generate_pin();
        let pin2 = PairingProtocol::generate_pin();
        assert_eq!(pin1.len(), 6);
        assert!(pin1.chars().all(|c| c.is_ascii_digit()));
        assert_ne!(pin1, pin2);
    }

    #[tokio::test]
    async fn test_hash_and_verify_pin() {
        let nonce = b"test-nonce";
        let pin = "482391";
        let hash = PairingProtocol::hash_pin(pin, nonce);
        assert!(PairingProtocol::verify_pin(pin, nonce, &hash));
        assert!(!PairingProtocol::verify_pin("000000", nonce, &hash));
    }

    #[tokio::test]
    async fn test_full_pairing_flow() {
        let dir = TempDir::new().unwrap();
        let client_identity = IdentityManager::generate(dir.path(), "client").await.unwrap();
        let server_identity = IdentityManager::generate(dir.path(), "server").await.unwrap();

        let client_challenge = PairingProtocol::create_challenge(&client_identity).unwrap();
        let (session, pin) = PairingProtocol::start_server(
            client_identity.public_key_bytes(),
            &client_challenge,
        ).unwrap();

        assert_eq!(pin.len(), 6);

        let pin_proof = PairingProtocol::hash_pin(&pin, &session.server_nonce);
        let (pin_proof_sig, _) = client_identity.sign_standard(&pin_proof);

        let result = PairingProtocol::confirm_server(
            &session, &pin, &pin_proof_sig, client_identity.public_key_bytes(),
        );
        assert!(result.is_ok());

        let result = PairingProtocol::confirm_server(
            &session, "000000", &pin_proof_sig, client_identity.public_key_bytes(),
        );
        assert!(result.is_err());
    }
}
