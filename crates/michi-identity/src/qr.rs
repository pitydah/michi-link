//! QRConnector: versioned out-of-band pairing URIs (contract v1).
//!
//! Canonical URI:
//!
//! ```text
//! michi://pair?format=michi-link-pairing&version=1
//!   &server_michi_id=<base64url>&server_public_key=<base64url>
//!   &session_id=<uuid>&expires_at=<RFC3339>&endpoint=<urlencoded>
//! ```
//!
//! The QR never contains private secrets, final tokens or any verifiable
//! PIN hash. The endpoint must use http/https.

use std::sync::Arc;
use uuid::Uuid;

use crate::error::IdentityError;
use crate::identity::IdentityManager;
use crate::types::PairingQr;

const URI_SCHEME: &str = "michi";
const URI_HOST: &str = "pair";
const QR_FORMAT: &str = "michi-link-pairing";
const QR_VERSION: u32 = 1;
/// Maximum total URI length accepted/emitted.
pub const MAX_QR_URI_LENGTH: usize = 1024;

/// Connector for out-of-band pairing via QR codes.
pub struct QRConnector {
    identity: Arc<IdentityManager>,
}

impl QRConnector {
    pub fn new(identity: Arc<IdentityManager>) -> Self {
        Self { identity }
    }

    /// Builds a versioned pairing URI for an active session.
    pub fn generate_pairing_uri(
        &self,
        session_id: Uuid,
        expires_at: chrono::DateTime<chrono::Utc>,
        endpoint: &url::Url,
    ) -> Result<String, IdentityError> {
        Self::build_uri(
            &self.identity.michi_id().to_base64url(),
            &self.identity.public_key_base64url(),
            session_id,
            &expires_at.to_rfc3339(),
            endpoint,
        )
    }

    /// Parses and validates a pairing URI. Returns the structured payload.
    ///
    /// The session remains the bearer of intent: confirming it over the
    /// endpoint requires the PIN, so no signature is embedded in the QR.
    pub fn parse_pairing_uri(uri: &str) -> Result<PairingQr, IdentityError> {
        if uri.len() > MAX_QR_URI_LENGTH {
            return Err(IdentityError::QrTooLarge);
        }
        let parsed = url::Url::parse(uri)
            .map_err(|e| IdentityError::UriParseFailed(format!("invalid URI: {}", e)))?;

        if parsed.scheme() != URI_SCHEME {
            return Err(IdentityError::UriParseFailed(format!(
                "invalid scheme: {}",
                parsed.scheme()
            )));
        }
        if parsed.host_str() != Some(URI_HOST) {
            return Err(IdentityError::UriParseFailed(format!(
                "invalid host: {:?}",
                parsed.host_str()
            )));
        }

        let get_param = |name: &str| -> Result<String, IdentityError> {
            parsed
                .query_pairs()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| {
                    IdentityError::UriParseFailed(format!("missing parameter: {}", name))
                })
        };

        let format = get_param("format")?;
        if format != QR_FORMAT {
            return Err(IdentityError::UriParseFailed(format!(
                "unsupported format: {}",
                format
            )));
        }
        let version: u32 = get_param("version")?
            .parse()
            .map_err(|_| IdentityError::UriParseFailed("invalid version".into()))?;
        if version != QR_VERSION {
            return Err(IdentityError::UriParseFailed(format!(
                "unsupported version: {}",
                version
            )));
        }

        let server_michi_id = get_param("server_michi_id")?;
        let server_public_key = get_param("server_public_key")?;
        let session_id: Uuid = get_param("session_id")?
            .parse()
            .map_err(|_| IdentityError::UriParseFailed("invalid session_id".into()))?;
        let expires_at = get_param("expires_at")?;
        chrono::DateTime::parse_from_rfc3339(&expires_at)
            .map_err(|_| IdentityError::UriParseFailed("invalid expires_at".into()))?;

        // The endpoint must be an http(s) URL.
        let endpoint = get_param("endpoint")?;
        let endpoint_url = url::Url::parse(&endpoint)
            .map_err(|e| IdentityError::UriParseFailed(format!("invalid endpoint: {}", e)))?;
        match endpoint_url.scheme() {
            "http" | "https" => {}
            _ => {
                return Err(IdentityError::UrlSchemeNotAllowed);
            }
        }

        Ok(PairingQr {
            format,
            version,
            server_michi_id,
            server_public_key,
            session_id,
            expires_at,
            endpoint: endpoint_url,
        })
    }

    fn build_uri(
        server_michi_id: &str,
        server_public_key: &str,
        session_id: Uuid,
        expires_at: &str,
        endpoint: &url::Url,
    ) -> Result<String, IdentityError> {
        match endpoint.scheme() {
            "http" | "https" => {}
            _ => {
                return Err(IdentityError::UrlSchemeNotAllowed);
            }
        }
        let uri = format!(
            "{}://{}?format={}&version={}&server_michi_id={}&server_public_key={}&session_id={}&expires_at={}&endpoint={}",
            URI_SCHEME,
            URI_HOST,
            QR_FORMAT,
            QR_VERSION,
            server_michi_id,
            server_public_key,
            session_id,
            urlencoding::encode(expires_at),
            urlencoding::encode(endpoint.as_str()),
        );
        if uri.len() > MAX_QR_URI_LENGTH {
            return Err(IdentityError::QrTooLarge);
        }
        Ok(uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn connector() -> (QRConnector, Arc<IdentityManager>) {
        let dir = TempDir::new().unwrap();
        let identity = Arc::new(IdentityManager::generate(dir.path(), "qr-device", "pw").unwrap());
        (QRConnector::new(identity.clone()), identity)
    }

    fn endpoint() -> url::Url {
        url::Url::parse("http://192.168.1.50:8400").unwrap()
    }

    #[test]
    fn test_roundtrip() {
        let (connector, identity) = connector();
        let session_id = Uuid::new_v4();
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(5);

        let uri = connector
            .generate_pairing_uri(session_id, expires_at, &endpoint())
            .unwrap();
        assert!(uri.starts_with("michi://pair?format=michi-link-pairing&version=1"));
        assert!(uri.len() <= MAX_QR_URI_LENGTH);

        let qr = QRConnector::parse_pairing_uri(&uri).unwrap();
        assert_eq!(qr.format, "michi-link-pairing");
        assert_eq!(qr.version, 1);
        assert_eq!(qr.session_id, session_id);
        assert_eq!(qr.server_michi_id, identity.michi_id().to_base64url());
        assert_eq!(qr.endpoint, endpoint());
        // The URI must not contain PIN material or final tokens.
        assert!(!uri.contains("pin"));
        assert!(!uri.contains("token"));
    }

    #[test]
    fn test_invalid_scheme_rejected() {
        let uri = "https://evil.com/pair?format=michi-link-pairing&version=1&server_michi_id=x&server_public_key=y&session_id=00000000-0000-0000-0000-000000000000&expires_at=2026-01-01T00%3A00%3A00Z&endpoint=http%3A%2F%2F192.168.1.50%3A8400";
        let err = QRConnector::parse_pairing_uri(uri).unwrap_err();
        assert!(
            matches!(err, IdentityError::UriParseFailed(_)),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_missing_params_rejected() {
        let uri = "michi://pair?format=michi-link-pairing&version=1";
        let err = QRConnector::parse_pairing_uri(uri).unwrap_err();
        assert!(
            matches!(err, IdentityError::UriParseFailed(_)),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_non_http_endpoint_rejected() {
        let (connector, _) = connector();
        let bad_endpoint = url::Url::parse("ftp://192.168.1.50:8400").unwrap();
        let err = connector
            .generate_pairing_uri(Uuid::new_v4(), chrono::Utc::now(), &bad_endpoint)
            .unwrap_err();
        assert!(
            matches!(err, IdentityError::UrlSchemeNotAllowed),
            "got {:?}",
            err
        );

        // And a crafted URI with ftp endpoint is rejected at parse time too.
        let uri = "michi://pair?format=michi-link-pairing&version=1&server_michi_id=x&server_public_key=y&session_id=00000000-0000-0000-0000-000000000000&expires_at=2026-01-01T00%3A00%3A00Z&endpoint=ftp%3A%2F%2F192.168.1.50%3A8400";
        let err = QRConnector::parse_pairing_uri(uri).unwrap_err();
        assert!(
            matches!(err, IdentityError::UrlSchemeNotAllowed),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_oversized_uri_rejected() {
        let (connector, _) = connector();
        let mut endpoint = endpoint();
        let long_path = "a".repeat(1200);
        endpoint.set_path(&long_path);
        let err = connector
            .generate_pairing_uri(Uuid::new_v4(), chrono::Utc::now(), &endpoint)
            .unwrap_err();
        assert!(matches!(err, IdentityError::QrTooLarge), "got {:?}", err);
    }

    #[test]
    fn test_bad_version_rejected() {
        let uri = "michi://pair?format=michi-link-pairing&version=2&server_michi_id=x&server_public_key=y&session_id=00000000-0000-0000-0000-000000000000&expires_at=2026-01-01T00%3A00%3A00Z&endpoint=http%3A%2F%2F192.168.1.50%3A8400";
        let err = QRConnector::parse_pairing_uri(uri).unwrap_err();
        assert!(
            matches!(err, IdentityError::UriParseFailed(_)),
            "got {:?}",
            err
        );
    }
}
