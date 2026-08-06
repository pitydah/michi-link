# Pairing Protocol

Trust On First Use (TOFU) pairing between a client and a server, protected by a **6-digit PIN** displayed by the server, with mandatory session limits.

## Session Rules

| Rule | Value |
|------|-------|
| Maximum duration | 5 minutes |
| Maximum attempts | 5 |
| Use | Single use — consumed after success |
| PIN | 6 digits, cryptographically random (server-side) |

## Flow

```
Client                                Server
  |                                     |
  |-- POST /pair/start ---------------->|  validate client key + michi_id
  |                                     |  generate PIN (6 digits, random)
  |                                     |  create session (5 min, 5 attempts,
  |                                     |  single use) + server random secret
  |<-- { session_id, expires_at, -------|  PIN shown on the server display
  |      server_michi_id,               |
  |      server_public_key }            |
  |                                     |
  |-- POST /pair/confirm -------------->|  validate, in order:
  |     { session_id, pin,              |  1. session exists
  |       public_key, michi_id }        |  2. not expired
  |                                     |  3. not consumed
  |                                     |  4. attempts remaining
  |                                     |  5. same client key + michi_id
  |                                     |  6. identity coherence
  |                                     |  7. PIN (constant-time, keyed)
  |<-- { token, expires_in } -----------|  token is opaque; secrets wiped
```

### POST /pair/start

The client sends its public key (and optionally its `michi_id`). When `michi_id` is provided it MUST match the identity derived from the key, otherwise the start is rejected with `PAIRING_KEY_MISMATCH`.

**Request:**

```json
{
  "public_key": "<client public key, base64>",
  "michi_id": "<client michi_id, base64url>"
}
```

`michi_id` is optional at start; `public_key` is required.

**Response (200):**

```json
{
  "session_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
  "expires_at": "2026-06-29T12:05:00Z",
  "server_michi_id": "<base64url 43 chars>",
  "server_public_key": "<base64>"
}
```

The 6-digit PIN is **shown on the server display**; it is never returned to the client over the network.

### POST /pair/confirm

The client confirms with the PIN entered by the user plus its own key material.

**Request:**

```json
{
  "session_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
  "pin": "482391",
  "public_key": "<client public key, base64>",
  "michi_id": "<client michi_id, base64url>"
}
```

**Validations, in order:**

1. Session exists (`PAIRING_NOT_FOUND` / invalid session).
2. Not expired → `PAIRING_EXPIRED` (the expired session is removed and can never be reused).
3. Not consumed → single use.
4. Attempts remaining; a wrong PIN decrements the counter.
5. Same client `public_key` as at start → `PAIRING_KEY_MISMATCH`.
6. Claimed `michi_id` must derive from the key and match the session → `PAIRING_KEY_MISMATCH`.
7. PIN verified with a keyed verifier and **constant-time comparison**.

**Response (200):**

```json
{
  "token": "<opaque bearer token>",
  "expires_in": 3600
}
```

The token is an **opaque bearer token** — NOT a JWT. It is a random value resolved server-side; no claims are embedded in the token itself.

### Success side effects

On success the session is marked consumed and the ephemeral secrets are wiped from memory (`server_secret` and `pin_verifier` zeroed). A consumed session can never confirm again.

## PIN Security

- The server generates a per-session random secret held **only in memory**.
- The verifier is `blake3(server_secret || pin)`.
- Because the secret never leaves the server, a 6-digit PIN **cannot be brute-forced offline** from anything exposed to the client (only 10^6 possibilities, but the verifier cannot even be computed without the secret).
- PIN comparison uses a **constant-time** check (subtle crate).
- `pin_hash` is never exposed anywhere, including the QR URI.

## Errors

| Error code | When |
|------------|------|
| `PAIRING_EXPIRED` | The session exceeded its 5-minute lifetime |
| `PAIRING_ATTEMPTS_EXCEEDED` | The 5-attempt budget was exhausted (session invalidated) |
| `PAIRING_KEY_MISMATCH` | Client key/michi_id inconsistent with the session |

All pairing errors use the canonical error envelope: `{"error": {"code", "message", "details"?, "request_id"?}}`.

## QR Pairing (out-of-band)

For camera-based pairing, the server encodes the session in a versioned URI:

```
michi://pair?format=michi-link-pairing&version=1&server_michi_id=<b64url>&server_public_key=<b64url>&session_id=<uuid>&expires_at=<RFC3339>&endpoint=<http(s)>
```

Constraints:

- Maximum URI length: **1024 characters**.
- `endpoint` must be `http` or `https`.
- The URI contains **no secrets**: no private key, no final token, no PIN hash. Confirming the session still requires the PIN, so the QR alone grants nothing.

Example:

```text
michi://pair?format=michi-link-pairing&version=1&server_michi_id=abc123...&server_public_key=A5B6...&session_id=9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d&expires_at=2026-06-29T12%3A05%3A00Z&endpoint=http%3A%2F%2F192.168.1.50%3A8400
```

## Token Refresh and Revocation

- Token renewal is handled by **`POST /api/v1/token/refresh`** (the old `/pair/refresh` path is retired — never use it).
- Device revocation is handled by **`POST /api/v1/devices/revoke`** (the old `DELETE /pair/device` path is retired — never use it).

## Receivers

Receivers (v1-lite) pair through the same canonical flow with auth strategy `RECEIVER_BUTTON` (physical button on the device) and `token_refresh: false`. See [docs/RECEIVERS_V1_LITE.md](RECEIVERS_V1_LITE.md).

## Reference implementation tests

The crate `crates/michi-identity` covers the pairing contract (`src/pairing.rs`):

| Test | Verifies |
|------|----------|
| `test_successful_confirm` | full happy path |
| `test_wrong_pin` | wrong PIN → `PinMismatch` |
| `test_expired_session` | expired session → `PAIRING_EXPIRED` |
| `test_sixth_attempt_rejected` | 5 wrong attempts exhaust the session; a 6th is impossible |
| `test_reuse_after_success_rejected` | single use |
| `test_different_public_key_rejected` | key swap → `PAIRING_KEY_MISMATCH` |
| `test_different_michi_id_rejected` | identity swap → `PAIRING_KEY_MISMATCH` |
| `test_claimed_michi_id_mismatch_at_start` | bad `michi_id` at start → `PAIRING_KEY_MISMATCH` |
| `test_nonexistent_session` | unknown session rejected |
| `test_consumed_session_after_success` | secrets wiped on success |
| `test_pin_not_verifiable_offline` | verifier is not `blake3(pin)`; needs the server secret |
| `test_concurrent_confirmations_single_winner` | exactly one concurrent confirm wins |
| `test_pin_format` | PIN is exactly 6 ASCII digits |

QR pairing (`src/qr.rs`): roundtrip, scheme/host validation, missing params, non-HTTP endpoints rejected, oversized URIs rejected, unsupported versions rejected.

## Related

- [docs/MICHI_IDENTITY.md](MICHI_IDENTITY.md) — identity, keys, `michi_id`
- [docs/DISCOVERY.md](DISCOVERY.md) — how devices find each other before pairing
- [docs/RECEIVERS_V1_LITE.md](RECEIVERS_V1_LITE.md) — receiver pairing (`RECEIVER_BUTTON`)
- [crates/michi-identity/](../crates/michi-identity/) — reference implementation
