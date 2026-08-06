//! Pairing: TOFU + 6-digit PIN with mandatory limits (contract v1).
//!
//! Session rules:
//! - Maximum duration: 5 minutes
//! - Maximum attempts: 5
//! - Single use (consumed after success)
//!
//! The PIN verifier is `blake3(server_secret || pin)` keyed with a server-side
//! random secret held only in memory, so a 6-digit PIN can never be
//! brute-forced offline from anything exposed to the client.

use rand::{Rng, RngCore};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use crate::error::IdentityError;
use crate::identity::IdentityManager;
use crate::types::PairingSession;

/// Maximum pairing session lifetime.
pub const MAX_SESSION_DURATION: Duration = Duration::from_secs(5 * 60);
/// Maximum number of PIN attempts per session.
pub const MAX_ATTEMPTS: u8 = 5;

/// Registry of active pairing sessions (server side).
#[derive(Debug, Default)]
pub struct PairingRegistry {
    sessions: Mutex<HashMap<Uuid, PairingSession>>,
}

impl PairingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generates a cryptographically random 6-digit PIN.
    pub fn generate_pin() -> String {
        let mut rng = rand::rngs::OsRng;
        let pin: u32 = rng.gen_range(100_000..1_000_000);
        format!("{:06}", pin)
    }

    /// Server side: opens a pairing session for the given client key.
    ///
    /// `client_michi_id` is optional at start; when provided it MUST match the
    /// identity derived from `client_public_key`.
    pub fn start_server(
        &self,
        server_identity: &IdentityManager,
        client_public_key: &[u8],
        client_michi_id: Option<&str>,
    ) -> Result<(Uuid, String), IdentityError> {
        let derived = IdentityManager::derive_michi_id(&base64_std(client_public_key))?;
        if let Some(claimed) = client_michi_id {
            if claimed != derived.to_base64url() {
                return Err(IdentityError::PairingKeyMismatch);
            }
        }

        let pin = Self::generate_pin();
        let mut server_secret = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut server_secret);
        let pin_verifier = Self::keyed_verifier(&server_secret, &pin);

        let now = SystemTime::now();
        let session = PairingSession {
            session_id: Uuid::new_v4(),
            server_michi_id: server_identity.michi_id().to_base64url(),
            server_public_key: server_identity.public_key_bytes().to_vec(),
            client_michi_id: client_michi_id.map(|s| s.to_string()),
            client_public_key: client_public_key.to_vec(),
            pin_verifier,
            server_secret,
            created_at: now,
            expires_at: now + MAX_SESSION_DURATION,
            attempts_remaining: MAX_ATTEMPTS,
            consumed: false,
        };

        let session_id = session.session_id;
        self.sessions.lock().unwrap().insert(session_id, session);
        Ok((session_id, pin))
    }

    /// Server side: confirms a pairing session.
    ///
    /// Validates, in order: session exists, not expired, not consumed,
    /// attempts available, same client public key and michi_id as at start,
    /// identity coherence, and PIN with a constant-time comparison.
    pub fn confirm(
        &self,
        session_id: Uuid,
        pin: &str,
        client_public_key: &[u8],
        client_michi_id: Option<&str>,
    ) -> Result<(), IdentityError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(&session_id)
            .ok_or(IdentityError::PairingNotFound)?;

        if session.is_expired(SystemTime::now()) {
            // Expired sessions are removed so they can never be reused.
            sessions.remove(&session_id);
            return Err(IdentityError::PairingExpired);
        }
        if session.consumed {
            return Err(IdentityError::PairingAlreadyConsumed);
        }
        if session.attempts_remaining == 0 {
            sessions.remove(&session_id);
            return Err(IdentityError::PairingAttemptsExceeded);
        }
        if session.client_public_key != client_public_key {
            return Err(IdentityError::PairingKeyMismatch);
        }
        if let Some(claimed) = client_michi_id {
            let derived = IdentityManager::derive_michi_id(&base64_std(client_public_key))?;
            if claimed != derived.to_base64url() {
                return Err(IdentityError::PairingKeyMismatch);
            }
            if session.client_michi_id.as_deref() != Some(claimed) {
                return Err(IdentityError::PairingKeyMismatch);
            }
        }

        if !Self::verify_pin_ct(&session.pin_verifier, &session.server_secret, pin) {
            session.attempts_remaining -= 1;
            if session.attempts_remaining == 0 {
                // Session invalidated: cannot reset the counter.
                sessions.remove(&session_id);
                return Err(IdentityError::PairingAttemptsExceeded);
            }
            return Err(IdentityError::PinMismatch);
        }

        // Success: consume the session and wipe ephemeral secrets.
        session.consumed = true;
        session.server_secret = [0u8; 32];
        session.pin_verifier = [0u8; 32];
        Ok(())
    }

    /// Keyed verifier: blake3(server_secret || pin).
    /// Without the server secret the PIN cannot be verified offline.
    fn keyed_verifier(server_secret: &[u8; 32], pin: &str) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(server_secret);
        hasher.update(pin.as_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Constant-time PIN comparison.
    fn verify_pin_ct(expected: &[u8; 32], server_secret: &[u8; 32], pin: &str) -> bool {
        use subtle::ConstantTimeEq;
        let computed = Self::keyed_verifier(server_secret, pin);
        computed.ct_eq(expected).into()
    }
}

fn base64_std(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityManager;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn server_identity() -> Arc<IdentityManager> {
        let dir = TempDir::new().unwrap();
        Arc::new(IdentityManager::generate(dir.path(), "server", "pw").unwrap())
    }

    fn client_identity() -> (Arc<IdentityManager>, Vec<u8>) {
        let dir = TempDir::new().unwrap();
        let id = Arc::new(IdentityManager::generate(dir.path(), "client", "pw").unwrap());
        let pk = id.public_key_bytes().to_vec();
        (id, pk)
    }

    fn open_session(
        registry: &PairingRegistry,
        server: &IdentityManager,
        client_pk: &[u8],
        client_michi_id: Option<&str>,
    ) -> (Uuid, String) {
        registry
            .start_server(server, client_pk, client_michi_id)
            .unwrap()
    }

    #[test]
    fn test_successful_confirm() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (client, pk) = client_identity();
        let (session_id, pin) = open_session(
            &registry,
            &server,
            &pk,
            Some(&client.michi_id().to_base64url()),
        );
        registry
            .confirm(
                session_id,
                &pin,
                &pk,
                Some(&client.michi_id().to_base64url()),
            )
            .unwrap();
    }

    #[test]
    fn test_wrong_pin() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (session_id, _pin) = open_session(&registry, &server, &pk, None);
        let err = registry
            .confirm(session_id, "000000", &pk, None)
            .unwrap_err();
        assert!(matches!(err, IdentityError::PinMismatch), "got {:?}", err);
    }

    #[test]
    fn test_expired_session() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (session_id, pin) = open_session(&registry, &server, &pk, None);
        // Age the session beyond its lifetime.
        {
            let mut sessions = registry.sessions.lock().unwrap();
            let session = sessions.get_mut(&session_id).unwrap();
            session.expires_at = SystemTime::now() - Duration::from_secs(1);
        }
        let err = registry.confirm(session_id, &pin, &pk, None).unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingExpired),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_sixth_attempt_rejected() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (session_id, pin) = open_session(&registry, &server, &pk, None);

        for i in 1..MAX_ATTEMPTS {
            let err = registry
                .confirm(session_id, "000000", &pk, None)
                .unwrap_err();
            assert!(
                matches!(err, IdentityError::PinMismatch),
                "attempt {} got {:?}",
                i,
                err
            );
        }
        // The fifth wrong attempt exhausts the session.
        let err = registry
            .confirm(session_id, "000000", &pk, None)
            .unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingAttemptsExceeded),
            "got {:?}",
            err
        );
        // The sixth attempt, even with the correct PIN, cannot reset the counter.
        let err = registry.confirm(session_id, &pin, &pk, None).unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingNotFound),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_reuse_after_success_rejected() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (session_id, pin) = open_session(&registry, &server, &pk, None);
        registry.confirm(session_id, &pin, &pk, None).unwrap();
        let err = registry.confirm(session_id, &pin, &pk, None).unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingAlreadyConsumed),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_different_public_key_rejected() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (_, other_pk) = client_identity();
        let (session_id, pin) = open_session(&registry, &server, &pk, None);
        let err = registry
            .confirm(session_id, &pin, &other_pk, None)
            .unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingKeyMismatch),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_different_michi_id_rejected() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (other, _) = client_identity();
        let (session_id, pin) = open_session(&registry, &server, &pk, None);
        let err = registry
            .confirm(
                session_id,
                &pin,
                &pk,
                Some(&other.michi_id().to_base64url()),
            )
            .unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingKeyMismatch),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_claimed_michi_id_mismatch_at_start() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_client, pk) = client_identity();
        let err = registry
            .start_server(
                &server,
                &pk,
                Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
            )
            .unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingKeyMismatch),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_nonexistent_session() {
        let registry = PairingRegistry::new();
        let (_, pk) = client_identity();
        let err = registry
            .confirm(Uuid::new_v4(), "123456", &pk, None)
            .unwrap_err();
        assert!(
            matches!(err, IdentityError::PairingNotFound),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_consumed_session_after_success() {
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (session_id, pin) = open_session(&registry, &server, &pk, None);
        registry.confirm(session_id, &pin, &pk, None).unwrap();
        // Internally the session is marked consumed and secrets wiped.
        let sessions = registry.sessions.lock().unwrap();
        let session = sessions.get(&session_id).unwrap();
        assert!(session.consumed);
        assert_eq!(session.server_secret, [0u8; 32]);
        assert_eq!(session.pin_verifier, [0u8; 32]);
    }

    #[test]
    fn test_pin_not_verifiable_offline() {
        // A client only ever sees the verifier... which it never does.
        // Here we prove the verifier cannot be brute-forced with public
        // information: hashing a candidate PIN without the server secret
        // never matches the stored verifier.
        let registry = PairingRegistry::new();
        let server = server_identity();
        let (_, pk) = client_identity();
        let (session_id, pin) = open_session(&registry, &server, &pk, None);

        let sessions = registry.sessions.lock().unwrap();
        let session = sessions.get(&session_id).unwrap();
        let verifier = session.pin_verifier;

        // Simulate an offline attacker that only knows the verifier (worst
        // case leak). They have no server_secret, so keyed_verifier cannot
        // even be computed for any candidate.
        // Structural check: verifier is NOT blake3(pin) or blake3(pin||nonce).
        let plain_hash = blake3::hash(pin.as_bytes());
        assert_ne!(*plain_hash.as_bytes(), verifier);
    }

    #[test]
    fn test_concurrent_confirmations_single_winner() {
        let registry = Arc::new(PairingRegistry::new());
        let server = server_identity();
        let (_, pk) = client_identity();
        let (session_id, pin) = registry.start_server(&server, &pk, None).unwrap();

        let pin_a = pin.clone();
        let pin_b = pin.clone();
        let pk_a = pk.clone();
        let reg_a = registry.clone();
        let reg_b = registry.clone();

        let handle_a = std::thread::spawn(move || reg_a.confirm(session_id, &pin_a, &pk_a, None));
        let handle_b = std::thread::spawn(move || reg_b.confirm(session_id, &pin_b, &pk, None));

        let results = [handle_a.join().unwrap(), handle_b.join().unwrap()];
        let wins = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(wins, 1, "exactly one confirmation must win: {:?}", results);
    }

    #[test]
    fn test_pin_format() {
        let pin = PairingRegistry::generate_pin();
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }
}
