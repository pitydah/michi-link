# Pairing Protocol

Trust On First Use (TOFU) pairing between a client and a server, protected by a **6-digit PIN displayed by the server**, with mandatory session limits and a bounded registry. This is the **only** pairing flow in contract v1: session + PIN + Ed25519 challenge.

## Session Rules

| Rule | Value |
|------|-------|
| Maximum duration | 5 minutes |
| Maximum attempts | 5 |
| Use | Single use — consumed after success |
| PIN | 6 digits (`^[0-9]{6}$`), cryptographically random (server-side) |
| Global active sessions | 1024 (`MAX_ACTIVE_SESSIONS`) |
| Sessions per source | 8 (`MAX_SESSIONS_PER_SOURCE`) |
| Sessions per identity | 4 (`MAX_SESSIONS_PER_IDENTITY`) |
| Pair starts per source | 20 per minute (`PAIR_START_RATE_LIMIT`) |

Expired sessions are cleaned up automatically (before create/confirm and via a public `cleanup_expired()` for schedulers); an expired session can never be reused.

## Flow

```
Client                                Server
  |                                     |
  |-- POST /pair/start ---------------->|  validate identity + challenge
  |     { device_name, device_type,     |  (Ed25519 signature over the raw
  |       roles, auth_strategy,         |   nonce bytes) — proves possession
  |       michi_id, public_key,         |  generate PIN (6 digits, random)
  |       challenge_nonce,              |  create session (5 min, 5 attempts,
  |       challenge_signature }         |  single use) + server random secret
  |<-- { session_id, expires_at, -------|  PIN shown on the server display,
  |      attempts_remaining,            |  NEVER sent over the wire
  |      server_michi_id,               |
  |      server_public_key }            |
  |                                     |
  |  (user reads the 6-digit PIN        |
  |   from the server display)          |
  |                                     |
  |-- POST /pair/confirm -------------->|  validate, in order:
  |     { session_id, pin,              |  1. session exists
  |       michi_id, public_key }        |  2. not expired
  |                                     |  3. not consumed
  |                                     |  4. attempts remaining
  |                                     |  5. same client key + michi_id
  |                                     |  6. identity coherence
  |                                     |  7. PIN (constant-time, keyed)
  |<-- { token, refresh_token?, -------|  token is opaque; secrets wiped
  |      expires_in, device_id,         |
  |      server_id }                    |
```

### POST /pair/start

The client sends its identity plus an **Ed25519 challenge**: a signature over the **raw bytes** of a random nonce. The server verifies the signature to prove possession of the secret key, then opens a session.

**Request:**

```json
{
  "device_name": "Michi Mobile",
  "device_type": "mobile",
  "roles": ["mobile_player", "remote_controller", "sync_client"],
  "auth_strategy": "ED25519_CHALLENGE",
  "michi_id": "97ryPKOLZ-JgVKQFc2ZuuSk0alWzxagdNILuDW26jEc",
  "public_key": "fDBBmExOH6h74KpGq2ckfDNN0Mzi7oMN4g_V2IKAR8Y",
  "challenge_nonce": "VFfZjzw8JeAM7-RFiTSrMA",
  "challenge_signature": "DTlMt9BYH_TnYgKAeGd8zTpza-w5b8BDm9AyIoAW2p0clD7JrzwN9cwPY5y48K14x_0z2TPq7-LTXdNTqmhr-w"
}
```

Wire encoding rules (strict):

- `michi_id` and `public_key` are **base64url without padding, exactly 43 chars** (`^[A-Za-z0-9_-]{43}$`).
- `challenge_nonce` is **base64url, at least 22 chars** (16 random bytes).
- `challenge_signature` is **base64url without padding, exactly 86 chars**.
- The characters `+`, `/`, `=` are **forbidden** in wire values. Hex and standard base64 are retired from the wire (standard base64 is only tolerated when reading legacy announces).
- `challenge_nonce` must not have been seen before (replay protection); a bad signature or nonce fails the start.
- When the claimed `michi_id` does not derive from `public_key`, the start is rejected with `PAIRING_KEY_MISMATCH`.

**Response (200):**

```json
{
  "session_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
  "expires_at": "2026-08-05T10:05:00Z",
  "attempts_remaining": 5,
  "server_michi_id": "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
  "server_public_key": "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ"
}
```

The 6-digit PIN is **shown on the server display**; it is never returned to the client and never travels over the network.

### POST /pair/confirm

The client confirms with the PIN entered by the user plus its own key material.

**Request:**

```json
{
  "session_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
  "pin": "482391",
  "michi_id": "97ryPKOLZ-JgVKQFc2ZuuSk0alWzxagdNILuDW26jEc",
  "public_key": "fDBBmExOH6h74KpGq2ckfDNN0Mzi7oMN4g_V2IKAR8Y"
}
```

**Validations, in order:**

1. Session exists → `PAIRING_NOT_FOUND` otherwise.
2. Not expired → `PAIRING_EXPIRED` (the expired session is removed and can never be reused).
3. Not consumed → single use (`PAIRING_ALREADY_CONSUMED` if already used).
4. Attempts remaining; a wrong PIN decrements the counter; exhaustion invalidates the session (`PAIRING_ATTEMPTS_EXCEEDED`).
5. Same client `public_key` as at start → `PAIRING_KEY_MISMATCH`.
6. Claimed `michi_id` must derive from the key and match the session → `PAIRING_KEY_MISMATCH`.
7. PIN verified with a keyed verifier and **constant-time comparison** (`PAIRING_PIN_MISMATCH` otherwise).

**Response (200):**

```json
{
  "token": "tok_michi_7f3a9c2e4b8d1f6a5c3e9b7d2a4f8c1e",
  "refresh_token": "tok_michi_refresh_9c4e2a7f1d8b3e6a5c0f2d4b8a1e7c3f",
  "expires_in": 3600,
  "device_id": "stable-device-id",
  "server_id": "stable-server-id"
}
```

- `token` is an **opaque bearer token** — NOT a JWT. It is a random value resolved server-side; no claims are embedded in the token itself.
- `refresh_token` is optional and present only when the server supports `/token/refresh` (e.g. Micro Server).

### Success side effects

On success the session is marked consumed and the ephemeral secrets are wiped from memory (`server_secret` and `pin_verifier` zeroed). A consumed session can never confirm again.

## PIN Security

- The server generates a per-session random secret held **only in memory**.
- The verifier is `blake3(server_secret || pin)` — keyed, so a 6-digit PIN **cannot be brute-forced offline** from anything exposed to the client (only 10^6 possibilities, but the verifier cannot even be computed without the secret).
- PIN comparison uses a **constant-time** check (subtle crate).
- The verifier is never exposed anywhere, including the QR URI.

## Registry limits

The pairing registry is bounded; exceeding any limit returns `RATE_LIMITED`:

| Limit | Value | Meaning |
|-------|-------|---------|
| Global active sessions | 1024 | Hard cap on open sessions |
| Sessions per source | 8 | Per source IP/key |
| Sessions per identity | 4 | Per client `michi_id` |
| Pair starts per minute | 20 | Per source |

`cleanup_expired()` runs automatically before every create/confirm and is exposed for scheduler-driven cleanup.

## Errors

| Error code | When |
|------------|------|
| `PAIRING_NOT_FOUND` | The session does not exist (or was cleaned up) |
| `PAIRING_EXPIRED` | The session exceeded its 5-minute lifetime |
| `PAIRING_ATTEMPTS_EXCEEDED` | The 5-attempt budget was exhausted (session invalidated) |
| `PAIRING_ALREADY_CONSUMED` | The session was already used successfully |
| `PAIRING_KEY_MISMATCH` | Client key/michi_id inconsistent with the session |
| `PAIRING_PIN_MISMATCH` | The PIN is wrong (401) |
| `RATE_LIMITED` | A registry limit was exceeded |

All pairing errors use the canonical error envelope: `{"error": {"code", "message", "details"?, "request_id"?}}`. See `schemas/error.schema.json` for the full set of 20 canonical codes.

## QR Pairing (out-of-band)

For camera-based pairing, the server encodes the session in a **versioned** URI:

```
michi://pair?format=michi-link-pairing&version=1&server_michi_id=<b64url>&server_public_key=<b64url>&session_id=<uuid>&expires_at=<RFC3339>&endpoint=<http(s)>
```

The JSON payload embedded in the QR is defined by `schemas/qr-pairing.schema.json` (`format: "michi-link-pairing"`, `version: 1`).

Constraints:

- Maximum URI length: **1024 characters**.
- `endpoint` must be `http` or `https`.
- The URI contains **no secrets**: no private key, no final token, no PIN verifier. Confirming the session still requires the PIN, so the QR alone grants nothing.

Example:

```text
michi://pair?format=michi-link-pairing&version=1&server_michi_id=QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays&server_public_key=KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ&session_id=9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d&expires_at=2026-08-05T10%3A05%3A00Z&endpoint=http%3A%2F%2F192.168.1.50%3A8400
```

## Token Refresh and Revocation

- Token renewal is handled by **`POST /api/v1/token/refresh`**.
- The legacy `/pair/refresh` path and `DELETE /pair/device` are **retired** and are not part of contract v1.

## Receivers

Receivers (v1-lite) pair through the canonical flow with auth strategy `RECEIVER_BUTTON` (physical button on the device) and `token_refresh: false`. The receiver is the pairing server; the pairing client is a controller (`device_type: server`). Decisions specific to receivers are frozen by [ADR-0001](adr/ADR-0001-receiver-v1-lite.md):

| Decision | Value |
|----------|-------|
| Physical window | A button press on the receiver opens a **120-second window**; outside it `POST /pair/start` responds `403 FORBIDDEN`. Rebooting closes the window; reopening replaces it and discards pending pairing sessions. |
| PIN display | The receiver generates a cryptographically random 6-digit PIN, shows it locally and never returns it over HTTP. |
| PIN transport | Under the **LAN trust model**, the PIN is sent by the client in `POST /pair/confirm` only. |
| Attempts | Max five failed PIN attempts per session; afterwards `429 RATE_LIMITED` and the session is consumed. |
| Token issuance | The token is **generated by the receiver**: 32 CSPRNG bytes, base64url without padding, returned exactly once. The receiver persists only the SHA-256 digest of the token. |
| Token lifetime | `expires_in: 0` — no automatic expiry; valid until revocation or factory reset. |
| Single use | The session is consumed after success; a second confirm responds `409 CONFLICT`. |
| Controller record | Stores `device_id`, `michi_id`, `public_key`, token digest, permissions, creation date and last activity. The PIN and the plaintext token are never stored. |

See [docs/RECEIVERS_V1_LITE.md](RECEIVERS_V1_LITE.md) for the full receiver profile.

## Schemas

| Schema | Contents |
|--------|----------|
| `schemas/pair-start.schema.json` | Canonical `/pair/start` request (challenge included; camelCase rejected by `additionalProperties: false`) |
| `schemas/pair-start-response.schema.json` | Session response (base64url 43-char identity fields) |
| `schemas/pair-confirm.schema.json` | Canonical `/pair/confirm` request (PIN + identity) |
| `schemas/pair-confirm-response.schema.json` | Opaque tokens + device/server identifiers |
| `schemas/qr-pairing.schema.json` | Versioned QR payload |

## Reference implementation tests

The crate `crates/michi-identity` covers the pairing contract (`src/pairing.rs`, `PairingRegistry`). The full crate suite is **88 tests**, clippy-clean:

| Test | Verifies |
|------|----------|
| `test_successful_confirm` | full happy path |
| `test_wrong_pin` | wrong PIN → `PAIRING_PIN_MISMATCH` |
| `test_expired_session` | expired session → `PAIRING_EXPIRED` |
| `test_sixth_attempt_rejected` | 5 wrong attempts exhaust the session; a 6th is impossible |
| `test_reuse_after_success_rejected` | single use → `PAIRING_ALREADY_CONSUMED` |
| `test_different_public_key_rejected` | key swap → `PAIRING_KEY_MISMATCH` |
| `test_different_michi_id_rejected` | identity swap → `PAIRING_KEY_MISMATCH` |
| `test_claimed_michi_id_mismatch_at_start` | bad `michi_id` at start → `PAIRING_KEY_MISMATCH` |
| `test_nonexistent_session` | unknown session rejected |
| `test_consumed_session_after_success` | secrets wiped on success |
| `test_pin_not_verifiable_offline` | verifier is not `blake3(pin)`; needs the server secret |
| `test_concurrent_confirmations_single_winner` | exactly one concurrent confirm wins |
| `test_pin_format` | PIN is exactly 6 ASCII digits |
| registry limits | 1024/8/4 caps and per-source rate limit (`RATE_LIMITED`) |
| `cleanup_expired` | expired sessions are removed automatically |

QR pairing (`src/qr.rs`): roundtrip, scheme/host validation, missing params, non-HTTP endpoints rejected, oversized URIs rejected, unsupported versions rejected.

## Related

- [docs/MICHI_IDENTITY.md](MICHI_IDENTITY.md) — identity, keys, `michi_id`
- [docs/DISCOVERY.md](DISCOVERY.md) — how devices find each other before pairing
- [docs/RECEIVERS_V1_LITE.md](RECEIVERS_V1_LITE.md) — receiver pairing (`RECEIVER_BUTTON`)
- [crates/michi-identity/](../crates/michi-identity/) — reference implementation

## Known gap: explicit revocation endpoint

Contract v1 has **no explicit unpair/revocation endpoint** (the legacy
`DELETE /pair/device` was retired and no `/devices/revoke` exists in the
current OpenAPI). Pairing state is bounded by design:

- sessions expire after 5 minutes and are cleaned up automatically;
- issued tokens are server-side resources revoked by the server's own
  token management (see `/token/refresh` and server token stores).

A dedicated revocation endpoint is future contract work and must go through
the contract change process before any implementation. Consumers must not
invent their own revocation paths.
