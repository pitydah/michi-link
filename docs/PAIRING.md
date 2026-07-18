# Pairing Protocol

The pairing flow has three stages: **start → confirm → token**.

## Flow

```
Client                   Server
  |                        |
  |-- POST /pair/start --->|
  |                        |  Generate code + temp token
  |<-- { code, token } ----|
  |                        |
  |-- POST /pair/confirm ->|
  |     { code }           |
  |                        |  Verify code, issue auth token
  |<-- { auth_token } -----|
  |                        |
  |-- (use auth_token) --->|
```

### Pair/Start

The client sends its device info. The server generates a 6-digit numeric code and a temporary token.

**Request:**
```
POST /pair/start
Content-Type: application/json

{
  "device_id": "uuid-v4",
  "device_name": "My Phone",
  "device_type": "mobile",
  "roles": ["controller", "player"],
  "capabilities": {
    "display": true,
    "streaming": ["mp3", "aac"]
  }
}
```

**Response (200):**
```json
{
  "code": "482391",
  "token": "temp_token_string",
  "expires_in": 300
}
```

### Pair/Confirm

The user enters the code shown on the target device. The server verifies it and returns a permanent auth token.

**Request:**
```
POST /pair/confirm
Content-Type: application/json
Authorization: Bearer <temp_token>

{
  "code": "482391"
}
```

**Response (200):**
```json
{
  "auth_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "bearer",
  "expires_in": 604800,
  "device_id": "server-uuid",
  "permissions": ["library.read", "playback.control", "..."]
}
```

## Token Format

Tokens are **opaque bearer tokens** (not JWT). They consist of a 256-bit random value generated from `OsRng`, hex-encoded (64 hex characters).

```text
Authorization: Bearer <64-hex-char-token>
```

The token does not contain embedded claims. Permissions and device identity are resolved server-side from the database using the SHA-256 hash of the token as lookup key.

### Token Properties

- **Format:** Opaque string (64 hex chars = 256 bits of entropy)
- **Storage:** SHA-256 hash in memory + database (`link_devices.token_hash`)
- **Expiration:** 7 days (configurable)
- **Refresh:** Token rotation invalidates the previous refresh token
- **Revocation:** Immediate (removed from in-memory store, flagged in database)

## Token Refresh

When a token is near expiry (within 10% of its lifetime), the client may refresh it:

```
POST /pair/refresh
Authorization: Bearer <expiring_token>

Response: { "auth_token": "<new_token>", "expires_in": 604800 }
```

## Device Revocation

A paired device can be revoked by:

1. **Server admin** via API: `DELETE /pair/device/{device_id}`
2. **Automatic**: if a device fails auth for 24+ hours, its token is pruned
3. **Manual re-pair**: a new pair/start invalidates the old session for the same `device_id`

## Security Considerations

- Pairing is only accepted from **local network** IPs (RFC 1918 / private ranges)
- No TLS is **required** on the local network, but it is **recommended** when available
- The pairing code is short-lived (5 minutes max) and single-use
- Tokens must be stored securely and never transmitted over WAN without TLS
