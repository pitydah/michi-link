# Michi Link — Canonical Interoperability Contract (R1 / R1.1 / R1.2 / R1.3 / R1.4)

Normative cross-repository contract governing secure, persistent, and verifiable
communications between `michi-micro-server`, `michi-mobile`, `michi-music-player`,
`michi-stream-standard`, and `michi-stream-hifi`.

> **Revision R1.4 (Server-First Mobile Readiness Freeze)**: Aligns
> `GET /api/v1/home/roster` directly with `home-device-roster.schema.json`
> (`version`, `home_id`, `devices`, `updated_at`), enforces exact no-leading-zero
> volume grammar parity (`^(0|[1-9][0-9]?|100)$`) between Rust and JSON Schema,
> separates `receiver_id` from `device_michi_id`, binds QR origin authorization
> to verified local server interfaces / advertised hostnames / operator config
> (never blanket RFC 1918 or `*.local`/`*.ts.net`), formalizes unified active
> queue coordination & degraded recovery (`503 QUEUE_DEGRADED`), and specifies
> forward/backward compatibility rules. Signed Home Membership, Device Auth,
> Server Confirmation, and AirCat Grant v1 canonical bytes remain **unchanged**.

> **Revision R1.3 (Additive Functional & Presence Convergence)**: Formalizes
> strict `set_volume` parsing (zero silent fallback to 80), active-queue item
> deletion and duplicate-safe semantics, separation of Home membership,
> authentication, and bounded live presence, and canonical QR pairing origin
> validation. Signed Home Membership, Device Auth, Server Confirmation, and
> AirCat Grant v1 canonical bytes remain **unchanged** (100% backward compatible).

> **Revision R1.2 (Additive Security Hardening)**: Tightens trusted reverse-proxy
> HTTPS header verification (`X-Forwarded-Proto` + RFC 7239 `Forwarded`) and
> strictly separates TLS listener readiness, X.509 certificate chain structure,
> SPKI pinning, and Android client trust verification. Signed Home Membership,
> mutual auth, and AirCat Grant v1 canonical bytes are **unchanged** (100%
> backward compatible).

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

## 5. Mandatory HTTPS/TLS Policy for Home Authority & `AIRCAT_STRICT` (R1.1 / R1.2)

### 5.1 Covered Endpoints (Fail-Closed HTTPS Required)
ALL Home enrollment, Home authentication, Home admin approval, and AirCat
original-file endpoints MUST enforce verified HTTPS/TLS transport (zero
cleartext HTTP fallback):
- `POST /api/v1/auth/challenge`, `POST /api/v1/auth/session`
- `POST /api/v1/home/enrollment/challenge`, `POST /api/v1/home/enrollment/request`,
  `GET /api/v1/home/enrollment/{requestId}/status`
- `GET /api/v1/home/enrollment/pending`, `POST /api/v1/home/enrollment/{requestId}/approve`,
  `POST /api/v1/home/enrollment/{requestId}/reject`, `POST /api/v1/home/members/enroll`
- `GET /api/v1/aircat/enrollment-qr`
- `GET /api/v1/aircat/manifest`, `POST /api/v1/aircat/authorize`,
  `GET /api/v1/aircat/download/{remoteId}`
- **Any `/api/v1/*` request presenting `Authorization: Bearer michi_home_*`**
  (Home sessions MUST NEVER travel over cleartext HTTP, even on library,
  playback, queue, or sync routes).

Normative transport refusals:
- TLS listener unconfigured or not bound -> `503 TLS_NOT_CONFIGURED`.
- Cleartext HTTP connection (or untrusted/contradictory/malformed proxy headers)
  -> `403 TLS_REQUIRED`.

### 5.1.1 Strict Trusted Reverse-Proxy HTTPS Policy & Precedence (R1.2)
An HTTP connection MUST NEVER gain HTTPS authority from client-controllable
headers. Transport verification follows a strict three-tier precedence:
1. **Direct verified TLS** (`AirCatTlsVerifiedConnection` inserted exclusively by
   the server's own TLS accept loop after a real handshake). Direct TLS is
   authoritative and ignores HTTP forwarding headers.
2. **Explicitly trusted reverse proxy HTTPS termination**, which requires ALL of:
   - `trust_proxy == true` in server configuration,
   - Peer socket identity present (`ConnectInfo<SocketAddr>`) and IP listed in
     explicit `trusted_proxies`,
   - Complete, strict parsing of **both** `X-Forwarded-Proto` and RFC 7239
     `Forwarded` whenever present — **silent preference for one header over the
     other is FORBIDDEN**.
3. **Fail-closed refusal** (`403 TLS_REQUIRED`).

Mandatory header validation rules when evaluating trusted reverse-proxy headers:
- Non-UTF-8 header values MUST cause immediate rejection (never silently dropped).
- Empty values, malformed RFC 7239 syntax (unclosed quotes, dangling escapes,
  missing `=`, missing `proto`, duplicate `proto` within an element), or any
  protocol value other than `https` (case-insensitive) — including `http`, `ws`,
  `wss`, or unknown tokens — MUST be rejected.
- Multi-hop chains containing any non-`https` hop MUST be rejected.
- When **both** `X-Forwarded-Proto` and `Forwarded` are present, BOTH MUST parse
  cleanly, ALL hops in BOTH headers MUST equal `https`, and both headers MUST
  report an identical hop count. Any contradiction (e.g.
  `X-Forwarded-Proto: https` with `Forwarded: proto=http`, or incompatible hop
  counts) MUST be rejected.

Edge proxy deployment boundary (Nginx, Caddy, Traefik):
- The reverse proxy MUST strip/overwrite client-supplied `X-Forwarded-Proto` and
  `Forwarded` headers and emit them strictly from the real inbound TLS socket.
- The backend HTTP listener MUST be firewalled/isolated from untrusted LAN/WAN
  clients so attackers cannot reach it directly or spoof trusted proxy IPs.

### 5.2 Strict Separation of TLS Readiness, X.509 Structure, SPKI Pinning & Android Trust (R1.2)
Implementations and diagnostic APIs (`GET /api/v1/aircat/status`) MUST separate
four independent properties and NEVER conflate them:
1. **TLS server readiness / leaf validity**: PEM parses cleanly, private key
   matches leaf certificate (`rustls`), leaf satisfies `notBefore <= now <= notAfter`,
   `SubjectAltName` (`DNS` or `IP`) matches `pinnedBaseUrl`, leaf ExtendedKeyUsage
   (when present) permits `serverAuth`, and the TLS listener is actively bound.
2. **X.509 certificate profile & chain verification** (`certificate_profile`,
   `chain_cryptographically_verified`):
   - `self_signed_verified`: single self-issued leaf whose cryptographic
     signature verifies against its own SubjectPublicKeyInfo.
   - `self_issued_unverified`: `issuer == subject`, but cryptographic
     self-signature verification did NOT succeed (equality of issuer and subject
     indicates *self-issued*, NOT *self-signed*).
   - `ca_issued_chain_present`: CA-issued leaf (`issuer != subject`) accompanied
     by ordered issuer/intermediate certificate(s) whose validity windows,
     `BasicConstraints (CA:TRUE)`, and cryptographic signatures verify against
     the presented chain tip.
   - `incomplete_chain`: CA-issued leaf without issuer/intermediate certs, OR
     unordered chain, OR expired/non-CA intermediate, OR failed signature check.
   - `unknown`: classification could not be determined.
3. **Client / Android trust verification** (`android_trust_verification_status`):
   - `not_verified`: server has NOT verified trust against Android's platform
     `X509TrustManager` trust store (default for CA-issued chains observed
     server-side — **a presented X.509 chain NEVER proves Android trust**).
   - `verified_with_explicit_trust_roots`: chain verified cryptographically
     against an explicit trust root bundle (still distinct from live Android
     device trust store unless that bundle is verified on Android).
   - `verification_failed`: known incompatible with default Android trust
     (e.g., `self_signed_verified`, `self_issued_unverified`, broken/expired
     intermediate chain, or failed explicit root check).
   - Legacy boolean `android_default_trust_compatible` MUST be `false` whenever
     `android_trust_verification_status` is `not_verified` or
     `verification_failed` (zero false positives). Note that `false` alongside
     `not_verified` indicates trust is unverified, not proven incompatible.
4. **RFC 7469 SPKI SHA-256 Pinning** (`tlsSpkiSha256`):
   - Exact match against the authorized enrollment anchor's SPKI hash.
   - Orthogonal to X.509 CA trust; never replaces X.509 verification.
   - `home_tls_spki_pinned` in `CanonicalServerCapabilities` reflects server TLS
     listener + valid leaf/SAN/SPKI readiness and MUST NOT be disabled solely
     because `android_trust_verification_status == "not_verified"` (clients that
     explicitly trust a private CA may still operate while Android fails closed
     when its trust store rejects the chain).

Supported client PKI deployment modes:
- **Mode A (Public CA — primary path for default Android `X509TrustManager` +
  OkHttp `CertificatePinner`)**: Let's Encrypt DNS-01 or Tailscale HTTPS
  (`fullchain.pem` + matching SAN + SPKI pin).
- **Mode B (Scoped Home Private CA — `NOT_IMPLEMENTED_IN_MOBILE`)**: optional
  future LAN mode pinning a Home-only `TrustManagerFactory` strictly to the Home
  origin. Installing trust-all `X509TrustManager` or disabling `HostnameVerifier`
  is STRICTLY FORBIDDEN.

### 5.3 Fail-Closed Revocation & Single Home Session Authority
- Home sessions (`michi_home_*`) MUST be managed by a dedicated Home session
  authority and MUST NOT be duplicated into legacy `SERVER_CODE` device token
  stores.
- Revocation checks MUST distinguish `Active`, `Revoked` (`403 MEMBER_REVOKED`),
  and `Indeterminate` (SQLite/storage error -> `503 REVOCATION_STORE_UNAVAILABLE`).
  Errors reading durable revocation state MUST NEVER fail open.

### 5.4 AirCat Original-File Invariants
- Protocol header `X-Michi-AirCat-Protocol: michi.aircat.original.v1` and
  `Cache-Control: no-store` (plus `no-transform` on downloads) are mandatory.
- Downloads serve exact original library bytes without transcoding, with strong
  quoted `ETag`, `Accept-Ranges: bytes`, HTTP 206 partial content / 416 / 304,
  receipt header `X-Michi-AirCat-Authorized-Grant-SHA256`, symlink refusal
  (`O_NOFOLLOW`), and TOCTOU file revision re-verification on every chunk.

## 6. Michi Control: Playback Volume & Queue Semantics (R1.3)

### 6.1 `set_volume` Normalization (`POST /api/v1/playback/control`)
- **Canonical field**: `volume` (JSON integer in `[0, 100]`). Volume `0` means
  real mute/silence; `100` is the contract maximum.
- **Legacy compatibility field**: `value` accepts either a JSON integer in
  `[0, 100]` or a strict canonical unsigned ASCII decimal string matching
  `^(0|[1-9][0-9]?|100)$` (`"0"`, `"1"`..`"100"` — leading zeros such as `"00"`,
  `"01"`, `"075"` are REJECTED identically by both JSON Schema and Rust).
- **Strict rejection rules (HTTP `400`, `error.code = "INVALID_REQUEST"`)**:
  - Missing or `null` volume (NEVER silently apply a fallback default such as `80`).
  - Negative numbers, values `> 100`, integer overflow beyond `u8`/`u32`, floats
    (`75.5`, `0.0`), booleans (`true`/`false`), objects/arrays (`422` at JSON decode).
  - Non-canonical or ambiguous text (`"00"`, `"075"`, `"75%"`, `"75.5"`, `"-1"`,
    `"+50"`, `"1000"`, `"eighty"`, `""`).
  - Contradictory `volume` and `value` fields when both are non-null (when both
    are valid and equal, e.g. `volume: 20` and `value: "20"`, the command succeeds).
- **State preservation invariant**: A rejected `set_volume` request MUST NEVER
  mutate the effective `PlaybackEngine` volume.

### 6.2 Active Queue Coordination, Deletion & Sequence Continuity
- **Single Coordinator**: All active queue mutations (`create`, `add/append`,
  `jump`, `transfer/replace`, `reorder`, `delete item`, `clear`, `delete queue`,
  `save/restore`) share one serialized coordinator with a monotonic `queue_version`
  and `BEGIN IMMEDIATE` SQLite transactions (committed before audio engine ops).
- **Explicit Degraded Recovery**: If a `PlaybackEngine` sync fails and the
  compensating SQLite rollback also fails, the coordinator MUST enter an explicit
  `degraded` state (`queue_degraded: true`) and reject new mutations fail-closed
  with HTTP `503 QUEUE_DEGRADED` until reconciliation restores SQLite ↔ engine
  coherence. Silent divergence is prohibited.
- **Canonical identifier**: Individual deletion (`DELETE /api/v1/queue/items/{item_id}`)
  operates on `queue_item_id`. Legacy fallback matching `track_id` MUST delete
  **at most one** entry (preferring the active slot if matched) and MUST NEVER
  bulk-delete duplicate occurrences of the same track.
- **Deleting the currently active item**:
  - Advances to the **next valid track in logical play order** (respecting active
    `shuffle` permutation and `repeat` policy) without jumping backward to index `0`.
  - If actively playing, transitions cleanly to that next track; if `paused`,
    `stopped`, or `idle`, selects the next track while preserving the non-playing
    state (NEVER auto-starts playback).
  - Under `repeat: "one"`, MUST NEVER replay a deleted item.
  - If the deleted active item was the last item in play order (or the only item):
    under `repeat: "all"` (with remaining items), wraps to the start of a new
    cycle; otherwise stops playback cleanly with no active track.
- **Deleting a non-active item**: Preserves the currently playing track, its
  playback position, its `playing`/`paused` state, and the upcoming logical
  sequence (`next` and `EOF`).

## 7. Home Presence & Canonical Roster Contract (R1.3 / R1.4)

`GET /api/v1/home/roster` MUST conform directly to `schemas/home-device-roster.schema.json`
(`version: 1`, `home_id`, `devices`, `updated_at`, plus optional `total` and
`presence_ttl_seconds`) and strictly separate four independent properties plus
identifier domains:
1. **Identifier domain separation**: `device_michi_id` (43-char base64url Ed25519
   BLAKE3 identity) and `receiver_id` (receiver registry key) are distinct domains
   and MUST NEVER be implicitly equated when resolving live streaming sessions.
2. **Membership (`membership_status`)**: `"enrolled"` or `"revoked"`. A durable
   membership record proves authorization to belong to Home, NOT live presence.
3. **Authentication (`authenticated`)**: `true` ONLY when the device currently
   holds an active, non-expired Home session or live authenticated receiver state
   (and is not revoked).
4. **Presence (`online`, `presence_state`, `status`)**: `"online"` / `"offline"` /
   `"revoked"` / `"unknown"`. `online: true` requires recent, verifiable live
   evidence within a bounded TTL (`180s` for authenticated Home session activity,
   live signed Whisker/Scent `verified_online` presence, or active receiver
   streaming session resolved by `receiver_id`). Persisted SQLite rows alone,
   pending enrollments, stale historical `last_seen_at`, or post-restart state
   MUST NEVER report `online: true`.
5. **Revocation (`revoked`)**: Revoked devices MUST always report `revoked: true`,
   `authenticated: false`, `online: false`, `membership_status: "revoked"`,
   `presence_state: "revoked"`, `status: "revoked"`. If the durable revocation
   store is unreadable, the endpoint MUST fail closed with HTTP `503 REVOCATION_STORE_UNAVAILABLE`.

## 8. QR Pairing Origin Policy & Protocol Separation (R1.3 / R1.4)

- **Origin policy (`POST /api/v1/pair/qr`)**:
  - Precedence: (1) validated explicit `server_url` (must comply with configured
    public URL/allowlist when set), (2) operator-configured canonical public URL
    (`MICHI_PUBLIC_URL` / `MICHI_PUBLIC_HTTPS_URL`), (3) verified trusted-proxy
    host/proto or direct listener host/scheme.
  - **Server-owned destination requirement (R1.4)**: Being an RFC 1918 IP,
    `*.local`, or `*.ts.net` is NOT sufficient by itself. The host MUST belong
    to THIS server via one of: (a) operator-configured public URL /
    `MICHI_ALLOWED_QR_HOSTS` / `MICHI_ADVERTISE_HOST` / `MICHI_TAILSCALE_HOST` /
    `MICHI_CORS_ORIGIN` / active TLS SAN, (b) an IP actually assigned to a local
    network interface of this host (`if_addrs` + loopback), or (c) an mDNS
    hostname actually advertised by this server. Another host on the same LAN
    (e.g. `192.168.1.30` when the server is `192.168.1.20`) MUST be rejected
    (`400 INVALID_REQUEST`).
  - Rejects non-`http`/`https` schemes (`javascript:`, `file:`, `data:`),
    userinfo (`user@host`), paths/queries/fragments, control characters/CRLF,
    multiple host values, contradictory/malformed proxy forwarding headers, and
    arbitrary external hosts.
- **Strict protocol separation**:
  - Legacy `SERVER_CODE` QR (`qr_protocol: "legacy_server_code_v1"`) and Home
    Enrollment QR (`michi.aircat.home.enrollment.v1`) are separate protocols.
  - Legacy pairing QRs MUST NEVER carry Home Root public keys, SPKI pins, or be
    promoted to a Home cryptographic trust anchor.

## 9. Protocol Versioning, Evolution & Compatibility Rules (R1.4)

- **Wire format naming**: All Michi Link v1 HTTP/JSON contracts use `snake_case`
  except AirCat Grant/Manifest v1 (`camelCase` frozen by golden vectors in
  `tests/vectors/interop/micro-mobile-interop-vectors.json`). Cryptographic IDs
  and keys are 43-char Base64URL without padding; Ed25519 signatures are 86-char
  Base64URL without padding; administrative IDs (`server_id`, `track_id`,
  `queue_item_id`, `request_id`, `challenge_id`) are RFC 4122 UUIDs; timestamps
  are RFC 3339 UTC strings (or explicit `*_ms` epoch milliseconds where named).
- **Forward compatibility (Newer Server ↔ Older Client)**:
  - Additive optional fields on response DTOs are non-breaking; clients MUST
    ignore unknown JSON fields (`ignoreUnknownKeys = true`).
  - Existing required fields and signed canonical byte layouts MUST NOT change
    within v1.
- **Backward compatibility (Older Server ↔ Newer Client)**:
  - Clients MUST inspect `GET /api/v1/server/info` and `GET /api/v1/capabilities`
    and MUST NOT assume a flow is supported simply because an endpoint exists or
    a field is absent. A disabled capability (`false`) or missing flag MUST
    disable the corresponding client UI/flow gracefully.
- **Degraded security / protocol mismatch**:
  - Incompatible `identity_scheme` (anything other than `"ed25519-blake3-v1"`),
    `BLAKE3(public_key) != michi_id`, `android_trust_verification_status == "verification_failed"`,
    `403 TLS_REQUIRED`, `503 TLS_NOT_CONFIGURED`, or `503 REVOCATION_STORE_UNAVAILABLE`
    MUST fail closed without silent downgrade to cleartext HTTP or legacy `SERVER_CODE`.
