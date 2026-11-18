# Michi Link — Canonical Interoperability Contract (R1)

Normative cross-repository contract governing secure, persistent, and verifiable
communications between `michi-micro-server`, `michi-mobile`, `michi-music-player`,
`michi-stream-standard`, and `michi-stream-hifi`.

## 1. Canonical Cryptographic Identity

Every Michi peer owns a persistent Ed25519 keypair generated once and persisted
across restarts and upgrades.

- `identity_scheme`: `"ed25519-blake3-v1"`
- `public_key`: raw 32-byte Ed25519 verifying key encoded in strict Base64URL
  without padding (43 ASCII characters, alphabet `[A-Za-z0-9_-]`).
- `michi_id`: `base64url_nopad(BLAKE3(raw_32_byte_public_key))` (43 ASCII
  characters).
- `michi_home_id`: `base64url_nopad(BLAKE3(raw_32_byte_home_root_public_key))`
  (43 ASCII characters).
- `server_id`: administrative UUID v4 persisted independently. `server_id` MUST
  NEVER replace `michi_id` in cryptographic verification, Home sessions, or
  AirCat grants.
- Private keys (`identity.json`, `home_root_authority.key`, `tls.key`) are NEVER
  published over HTTP, SSE, WebSocket, mDNS, or UDP discovery, and MUST be
  stored with owner-only permissions (`0600` on POSIX).

### Verification Surface Coherence

1. `GET /api/v1/server/info` MUST publish `identity_scheme`, `michi_id`,
   `public_key`, and (when part of a Home) `michi_home_id`, all matching the
   persisted Ed25519 identity.
2. Signed Whisker UDP multicast announcements (`224.0.0.167:53318`) MUST carry
   the identical `michi_id`, `public_key`, and `michi_home_id`, signed by the
   same Ed25519 key.
3. mDNS TXT records (`_michi-link._tcp.local.`) MUST carry the identical
   `michi_id` and `michi_home_id`.
4. Discovery payloads (Whisker / mDNS) are strictly **provisional** until
   verified cryptographically against the Home Root Authority or pinned TLS
   identity. Clients MUST NEVER transfer bearer tokens or credentials
   automatically when a peer changes IP address or host.

## 2. Canonical Capability Registry

Single normative capability registry exported by `michi_identity::capabilities`:

| Capability Flag | Meaning | Required for AirCat Readiness |
| --- | --- | --- |
| `library` | Library browse/tracks/albums/artists | No |
| `search` | Library search | No |
| `streaming` | Audio streaming endpoints | No |
| `download` | Standard library track download | No |
| `sync` | Library synchronization host | No |
| `sync_manifest` | Delta/manifest sync endpoints | No |
| `artwork` | Cover artwork retrieval | No |
| `playlists` | Playlist management | No |
| `playback` | Remote playback state & Michi Control | No |
| `queue` | Playback queue manipulation | No |
| `events` | SSE / WebSocket event stream | No |
| `receivers` | Receiver management | No |
| `multiroom` | Rooms and multiroom groups | No |
| `aircat_original_v1` | AirCat original-file v1 protocol | **Yes** |
| `home_tls_spki_pinned` | Real HTTPS/TLS listener with RFC 7469 SPKI pin active | **Yes** |
| `aircat_manifest_v1` | `GET /api/v1/aircat/manifest` ready over HTTPS | **Yes** |
| `aircat_authorize_v1` | `POST /api/v1/aircat/authorize` ready over HTTPS | **Yes** |
| `aircat_original_download_v1` | `GET /api/v1/aircat/download/{remoteId}` ready over HTTPS | **Yes** |

Normative rules:
- `GET /api/v1/server/info`, `GET /api/v1/capabilities`, and Whisker discovery
  MUST project from the same runtime capability snapshot.
- All five AirCat flags MUST be `false` (and omitted from enabled lists) unless
  the HTTPS/TLS listener with valid certificate, private key, and SPKI pin is
  actively bound and serving.
- **Capability advertisement is NOT authorization.** Every AirCat endpoint
  enforces TLS transport, verified `HOME_MEMBERSHIP` session, non-revocation,
  and signed Ed25519 grant validation independently.

## 3. Authentication Profiles & Trust Architecture V2

### 3.1 Legacy General Pairing (`SERVER_CODE`)
- Retained for compatibility with existing `michi-music-player` and general
  library/playback/sync endpoints (`/api/v1/pair/start`, `/api/v1/pair/confirm`,
  `/api/v1/token/refresh`).
- Legacy `SERVER_CODE` device tokens MUST NEVER be upgraded implicitly into
  `HOME_MEMBERSHIP` sessions and MUST be rejected (`403 HOME_SESSION_REQUIRED`)
  on `AIRCAT_STRICT` routes.

### 3.2 Authorized Home Membership Bootstrap (Admin-Approved + Ed25519 PoP)
The enrollment QR (`GET /api/v1/aircat/enrollment-qr`, type
`michi.aircat.home.enrollment.v1`) distributes ONLY the public trust anchor
(`homeId`, `authorityPublicKeyBase64Url`, `sourceMichiId`, `pinnedBaseUrl`,
`tlsSpkiSha256`, short-lived nonce). Scanning the QR NEVER grants Home
membership by itself and there is NO physical-button enrollment.

Bootstrap sequence:
1. `POST /api/v1/home/enrollment/challenge` — client sends
   `{ purpose: "home_enrollment", home_id, client_michi_id, client_public_key }`. Server verifies
   `client_michi_id == BLAKE3(client_public_key)` and non-revocation, returning
   `{ challenge_id, challenge_nonce, home_id, server_michi_id, server_public_key, expires_in }`.
2. `POST /api/v1/home/enrollment/request` — client signs
   `michi-link-home-enrollment-pop-v1 || home_id || server_michi_id || client_michi_id || client_public_key || challenge_id || challenge_nonce`
   with its Ed25519 private key and submits the request. Server consumes the
   challenge, verifies PoP, and queues a **pending** enrollment (`202 Accepted`)
   returning `{ request_id, status: "pending", home_id, client_michi_id, expires_at, claim_token }`.
3. Administrator explicitly reviews `GET /api/v1/home/enrollment/pending` and
   invokes `POST /api/v1/home/enrollment/{requestId}/approve` (or `/reject`).
   Only authenticated administrator sessions (`is_admin == true`) may approve.
   Approval issues a signed `MichiMembership` via `HomeRootAuthority` and
   persists the member in the Home roster.
4. Client polls `GET /api/v1/home/enrollment/{requestId}/status` presenting
   header `X-Michi-Enrollment-Claim: <claim_token>` and persists the approved
   `MichiMembership` + `home_root_public_key`.

### 3.3 Mutual Authentication, Renewal, and Revocation (`HOME_MEMBERSHIP`)
1. `POST /api/v1/auth/challenge` -> `{ challenge_id, challenge_nonce, server_michi_id, server_public_key, expires_in }`.
2. Client signs:
   `michi-link-device-auth-v1 || home_id || server_michi_id || client_michi_id || challenge_id || challenge_nonce`.
3. `POST /api/v1/auth/session` -> server verifies:
   - `client_michi_id == membership.device_michi_id`,
   - `membership` signature under `HomeRootAuthority` + non-revocation (both
     in-memory and persisted SQLite `home_revocations`),
   - `client_signature` over the single-use challenge.
   Server issues opaque `michi_home_*` bearer token (TTL 3600s) and signs
   `server_signature` over:
   `michi-link-server-auth-v1 || home_id || server_michi_id || client_michi_id || challenge_id || session_token`.
4. Client MUST verify `server_membership` and `server_signature` using
   `verify_server_auth_session` before trusting the session.
5. **Renewal / Reconnection**: repeat `POST /api/v1/auth/challenge` +
   `POST /api/v1/auth/session` using the persisted `MichiMembership`.
6. **Immediate Revocation**: `POST /api/v1/home/revoke` (admin only) persists a
   signed `HomeDeviceRevocation`, marks the member revoked in persistent
   storage, invalidates all active Home sessions immediately, rejects new
   challenges/sessions, and aborts any in-flight AirCat downloads at chunk
   boundaries.

## 4. Identity Header Policy (`X-Michi-Device-Id`)

- Authenticated Home sessions bind the request principal exclusively to the
  cryptographically verified `client_michi_id`.
- `X-Michi-Device-Id` NEVER grants or substitutes identity.
- On `HOME_MEMBERSHIP` sessions:
  - Omitting `X-Michi-Device-Id` or sending `X-Michi-Device-Id: <client_michi_id>`
    is valid.
  - Sending any other value (including another device's Michi ID or an unrelated
    UUID) MUST be rejected with `403 IDENTITY_MISMATCH`.
  - In `POST /api/v1/aircat/authorize` and `GET /api/v1/aircat/download/{remoteId}`,
    `destinationMichiId` MUST equal the verified `client_michi_id`
    (otherwise `403 WRONG_AUDIENCE`).

## 5. `AIRCAT_STRICT` Transport & Original-File Invariants

- All AirCat transfer endpoints (`/api/v1/aircat/manifest`,
  `/api/v1/aircat/authorize`, `/api/v1/aircat/download/{remoteId}`) enforce
  **fail-closed HTTPS/TLS**:
  - If TLS is unconfigured or listener is not ready -> `503 TLS_NOT_CONFIGURED`.
  - If request arrives over cleartext HTTP -> `403 TLS_REQUIRED`.
  - Zero cleartext fallback.
- Certificate MUST carry valid X.509 v3 SAN matching the `pinnedBaseUrl`
  host/IP (so Android `X509TrustManager` + hostname verification succeed in
  addition to RFC 7469 SPKI SHA-256 pinning).
- Protocol header `X-Michi-AirCat-Protocol: michi.aircat.original.v1` and
  `Cache-Control: no-store` (plus `no-transform` on downloads) are mandatory.
- Downloads serve exact original library bytes without transcoding, with strong
  quoted `ETag`, `Accept-Ranges: bytes`, HTTP 206 partial content / 416 / 304,
  receipt header `X-Michi-AirCat-Authorized-Grant-SHA256`, symlink refusal
  (`O_NOFOLLOW`), and TOCTOU file revision re-verification on every chunk.
