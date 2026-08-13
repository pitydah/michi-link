# Receivers — v1-lite Protocol

> Frozen by [ADR-0001](adr/ADR-0001-receiver-v1-lite.md). Michi Link is the normative source for the v1-lite receiver profile; Michi Music Stream implements it without adapters, aliases or receiver-local schemas.

## What is v1-lite?

v1-lite is the receiver profile of the Michi Link contract, designed for physical audio receivers (constrained hardware). A receiver receives PCM audio over a single RTP/UDP session and converts it to physical output. A client drives the receiver exclusively through the canonical contract below.

**Receiver contract summary:**

| Aspect | Value |
|--------|-------|
| `api_version` | `v1-lite` |
| `service` | `michi-stream-standard` or `michi-stream-hifi` |
| `roles` | exactly `["audio_receiver"]` |
| Identity | Persistent Ed25519; `michi_id` derived from `public_key`, never equal to `server_id` |
| `auth.required` | `true` |
| `auth.strategy` | `RECEIVER_BUTTON` (physical button) |
| `auth.token_refresh` | `false` |
| Audio | `rtp_udp`, `pcm_s16le`, 48 kHz, 16-bit, stereo, 10 ms packets, payload type 97 |
| `buffer_ms` | `50..500` |
| Sessions | Exactly one active session per receiver |
| Heartbeat | Every 10 seconds; renews a 30-second lease |
| Pairing window | 120 seconds opened by a physical button press |

## Canonical routes

All routes start with `/api/v1`. All JSON bodies use `snake_case`, UTF-8 and `Content-Type: application/json`. Except for `204` responses, every error uses the canonical `Error` schema.

| Method | Route | Success | Auth | Feature |
|--------|-------|--------:|------|---------|
| `GET` | `/api/v1/server/info` | `200` | none | always |
| `POST` | `/api/v1/pair/start` | `201` | none; physical window open | always |
| `GET` | `/api/v1/pair/status` | `200` | none; `session_id` query | always |
| `POST` | `/api/v1/pair/confirm` | `200` | none; pairing session | always |
| `POST` | `/api/v1/receiver-lite/session` | `201` | Bearer | `session` |
| `GET` | `/api/v1/receiver-lite/session` | `200` | Bearer | `session` |
| `PATCH` | `/api/v1/receiver-lite/session` | `200` | Bearer + session | `session` |
| `DELETE` | `/api/v1/receiver-lite/session` | `204` | Bearer + session | `session` |
| `POST` | `/api/v1/receiver-lite/heartbeat` | `200` | Bearer + session | `heartbeat` |
| `PUT` | `/api/v1/receiver-lite/now-playing` | `204` | Bearer + session | `now_playing` optional |
| `GET` | `/api/v1/receiver-lite/diagnostics` | `200` | Bearer | `diagnostics` optional |
| `GET` | `/api/v1/receiver-lite/firmware` | `200` | Bearer | `ota` optional |
| `POST` | `/api/v1/receiver-lite/firmware` | `202` | Bearer + OTA permission | `ota` optional |

## Receiver info

`GET /api/v1/server/info` returns the canonical receiver description. Standard and Hi-Fi differ only in `service` until further certification exists:

```json
{
  "service": "michi-stream-standard",
  "name": "Michi Stream Cocina",
  "server_id": "550e8400-e29b-41d4-a716-446655440000",
  "version": "0.3.0",
  "api_version": "v1-lite",
  "roles": ["audio_receiver"],
  "identity_scheme": "ed25519-blake3-v1",
  "michi_id": "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
  "public_key": "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ",
  "auth": {
    "required": true,
    "strategy": "RECEIVER_BUTTON",
    "token_refresh": false
  },
  "features": {
    "session": true,
    "heartbeat": true,
    "volume": true,
    "now_playing": true,
    "diagnostics": true,
    "ota": true
  },
  "audio": {
    "transports": ["rtp_udp"],
    "codecs": ["pcm_s16le"],
    "sample_rates": [48000],
    "bit_depths": [16],
    "channels": [2],
    "packet_ms": [10],
    "payload_types": [97],
    "buffer_ms_min": 50,
    "buffer_ms_max": 500
  }
}
```

Rules:

- `server_id` is a stable UUID v4, generated once and persisted in NVS.
- `michi_id` derives from the public key; it is not equal to `server_id`.
- The three identity fields are mandatory for `michi-stream-*`.
- `roles` contains exactly one element: `audio_receiver`.
- `version` is the firmware version; no `firmware_version` or `michi_link_version` fields.
- A feature is `true` only when its handler is registered and has a positive test.
- `audio` declares reproducible capability, not the DAC's theoretical capability.
- Reject additional properties in the schema.

## Pairing

Receivers pair through the canonical flow with auth strategy `RECEIVER_BUTTON`:

- A physical button press opens a **120-second window**. Rebooting closes it; reopening replaces the previous window and discards pending pairing sessions. Outside the window, `POST /pair/start` responds `403 FORBIDDEN`.
- The receiver generates a cryptographically random 6-digit PIN, shows it locally and never returns it over HTTP. Under the LAN trust model, the client sends the PIN in `POST /pair/confirm` only.
- Max five failed PIN attempts per session; afterwards `429 RATE_LIMITED` and the session is consumed.
- The pairing token is issued by the receiver: 32 CSPRNG bytes, base64url without padding, returned once. The receiver persists only the SHA-256 digest. `expires_in: 0` means no automatic expiry — valid until revocation or factory reset.
- The session is consumed after success; a second confirm responds `409 CONFLICT`.

See [docs/PAIRING.md](PAIRING.md) for the full flow and limits.

## Audio session

### `POST /api/v1/receiver-lite/session`

Creates the receiver's audio session. All fields are mandatory; `additionalProperties: false`:

```json
{
  "transport": "rtp_udp",
  "codec": "pcm_s16le",
  "sample_rate": 48000,
  "bit_depth": 16,
  "channels": 2,
  "packet_ms": 10,
  "buffer_ms": 120,
  "payload_type": 97,
  "ssrc": 305419896,
  "volume": 70
}
```

| Field | Admitted value/range |
|-------|----------------------|
| `transport` | exactly `rtp_udp` |
| `codec` | exactly `pcm_s16le` |
| `sample_rate` | exactly `48000` |
| `bit_depth` | exactly `16` |
| `channels` | exactly `2` |
| `packet_ms` | exactly `10` |
| `buffer_ms` | integer `50..500` |
| `payload_type` | exactly `97` |
| `ssrc` | unsigned integer `1..4294967295` |
| `volume` | integer `0..100` |

- Invalid values are never rounded or corrected: respond `400 INVALID_REQUEST` with `details.field`.
- If a session is already active, respond `409 CONFLICT`.
- The receiver picks a free UDP port in `49152..65535`. The RTP source IP is fixed to the TCP IP of the HTTP request; `source_ip` is not accepted in JSON.
- Audio starts only after socket, buffer and engine are reserved successfully.

**Response `201 Created`:**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "session_token": "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
  "lease_seconds": 30,
  "effective": {
    "transport": "rtp_udp",
    "codec": "pcm_s16le",
    "sample_rate": 48000,
    "bit_depth": 16,
    "channels": 2,
    "packet_ms": 10,
    "buffer_ms": 120,
    "payload_type": 97,
    "ssrc": 305419896,
    "stream_port": 55300,
    "volume": 70
  }
}
```

### Accepted RTP

- RTP v2, no CSRC, no extension, no padding.
- Payload type exactly `97`; SSRC exactly the negotiated one (no first-packet-wins).
- Source IPv4 exactly the one inferred at session creation.
- PCM little-endian interleaved L/R, 16-bit, 48 kHz. A 10 ms packet carries 480 frames, 960 samples, 1920 payload bytes.
- Packets with wrong source, PT, SSRC or size are rejected and counted. Sequence wrap is handled; loss and reordering are detected without closing the session.

### `GET /api/v1/receiver-lite/session`

**Response `200 OK`** when a session exists (fields as implemented; `state` is `starting`, `playing`, `paused` or `stopping`):

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "state": "playing",
  "lease_remaining_ms": 24500,
  "volume": 70,
  "paused": false,
  "stream_port": 55300,
  "ssrc": 305419896,
  "packets_received": 1250,
  "packets_rejected": 0,
  "packets_lost": 0,
  "underruns": 0
}
```

`session_token` is never returned. No session: `404 NOT_FOUND`.

### `PATCH /api/v1/receiver-lite/session`

Updates at least one property: `volume` `0..100` and/or `paused` boolean.

```json
{
  "volume": 55,
  "paused": true
}
```

**Response `200 OK`:** the same status body as `GET` after applying the change. There is no separate volume endpoint.

### `DELETE /api/v1/receiver-lite/session`

- No body. Idempotent success for the authenticated session: `204`.
- Wrong session token: `401 UNAUTHORIZED`. No session: `404 NOT_FOUND`.
- Close: stop accepting RTP, silence, stop the engine, release buffers/socket and delete the session token from RAM.

Internal state moves `idle → starting → playing` on creation; on partial failure the socket/buffer/engine are released and the state returns to `idle` — no ghost sessions. `PATCH` toggles `paused`; `DELETE` or lease expiry move to `stopping` and then `idle` after resources are released.

## Heartbeat and lease

`POST /api/v1/receiver-lite/heartbeat` every **10 seconds**:

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "sequence": 7,
  "sent_at_ms": 1786564800000
}
```

- `sequence`: unsigned integer, strictly increasing within the session.
- `sent_at_ms`: Unix epoch milliseconds, informative — not used for the local timeout.
- A valid heartbeat renews the lease to 30 seconds. Repeated or older heartbeats respond `409 CONFLICT` and do not renew.

**Response `200 OK`:**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "status": "alive",
  "lease_seconds": 30,
  "receiver_uptime_ms": 918273
}
```

The watchdog uses the monotonic clock. At 30 seconds without a valid heartbeat the receiver runs the same safe close as `DELETE`, increments `lease_expirations` and returns to `idle` — even if RTP packets keep arriving.

## Optional extensions

`now-playing` (`PUT /api/v1/receiver-lite/now-playing`), diagnostics (`GET /api/v1/receiver-lite/diagnostics`) and OTA (`GET`/`POST /api/v1/receiver-lite/firmware`) are optional extensions announced through feature flags. A feature flag is `true` only when the handler is registered and tested.

## Server-side Receiver Management

The server tracks receivers through:

- `GET /api/v1/receivers` — list discovered and registered receivers.
- `POST /api/v1/receivers/discover` — trigger network discovery (`timeout` query param, 1–30 s, default 5).

There is no `POST /api/v1/receivers/announce` endpoint: presence is announced exclusively through the multicast/mDNS discovery channel.

## Hardware notes

### Standard

| Characteristic | Specification |
|----------------|---------------|
| Audio output | Stereo jack 3.5mm |
| DAC | Integrated in SoC |
| RAM | ~512 KB |
| Storage | ~4 MB |
| Certified format | 16-bit / 48 kHz (`pcm_s16le`) |
| Connectivity | Wi-Fi 802.11 b/g/n |
| Typical use | Secondary rooms, bathroom |

### Hi-Fi

| Characteristic | Specification |
|----------------|---------------|
| Audio output | RCA (L/R) + jack 3.5mm |
| DAC | PCM5242 or better |
| RAM | ~4 MB |
| Storage | ~16 MB |
| Certified format | 16-bit / 48 kHz (`pcm_s16le`) — same certified audio as Standard; higher formats are not part of this convergence |
| Connectivity | Wi-Fi 802.11 b/g/n + Ethernet |
| Typical use | Living room, main setup |

## Not supported

The following are out of scope for this convergence and must not be reintroduced:

**Legacy receiver routes**

- `/api/v1/receiver/info`
- `/api/v1/receiver/session/start`
- `/api/v1/receiver/session/stop`
- `/api/v1/receiver/pair/*`
- `/api/v1/receiver-lite/volume` (replaced by `PATCH /receiver-lite/session`)
- `/api/v1/receiver-lite/info` and `/api/v1/receiver-lite/config`

**Music library**

- Playlists, library browsing, search, `track_id`, song download
- Autonomous playback from URL or local storage

**Codecs and transports**

- Opus, FLAC, MP3, PCM 24-bit (`pcm_s24le`), 96 kHz
- AirPlay, Spotify Connect, Bluetooth, HDMI, optical

**System**

- Multiroom, clock sync, RTCP, drift compensation
- TLS, PAKE, Secure Boot or Flash Encryption before the production packages
- More than one active session
- A parallel legacy API

## Related

- [docs/adr/ADR-0001-receiver-v1-lite.md](adr/ADR-0001-receiver-v1-lite.md) — frozen interoperability decisions
- [docs/DISCOVERY.md](DISCOVERY.md) — announce and discovery contract
- [docs/PAIRING.md](PAIRING.md) — pairing flow and limits
- [docs/AUTH_PROFILES.md](AUTH_PROFILES.md) — `RECEIVER_BUTTON` strategy
- [docs/MICHI_LINK_API_V1.md](MICHI_LINK_API_V1.md) — full API reference
