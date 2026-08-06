# Receivers — v1-lite Protocol

## What is v1-lite?

v1-lite is a subset of the v1 protocol designed for physical audio receivers (constrained hardware). Receivers do not manage libraries, playlists, search, sync, or complex state — they only receive audio and convert it to physical output.

**Receiver contract summary:**

| Aspect | Value |
|--------|-------|
| `api_version` | `"v1-lite"` |
| `service` | `michi-stream-standard` or `michi-stream-hifi` |
| `roles` | `["audio_receiver"]` |
| `auth.required` | `true` |
| `auth.strategy` | `RECEIVER_BUTTON` (physical button) |
| `auth.token_refresh` | `false` |
| Codecs (standard) | `pcm_s16le` |
| Codecs (hi-fi) | `pcm_s16le`, `pcm_s24le` |
| Heartbeat | every 10 seconds |
| Offline timeout | 30 seconds without heartbeat → `offline` |
| Example HTTP port | `8600` (receiver's own API) |

A receiver **never declares**: library, playlists, search, sync, storage, autonomous playback, transcoding, rooms, or token refresh. No codec other than `pcm_s16le` / `pcm_s24le` is advertised (no opus, no flac).

## Discovery

Receivers announce over the canonical discovery channel:

- **UDP multicast** `224.0.0.167:53318`, JSON UTF-8, max 8 KiB, every 30 seconds; offline after 90 seconds.
- **mDNS** `_michi-link._tcp.local` with the minimal TXT set (`device_id`, `service`, `api_version`, `roles`, `auth_strategy`).

See [docs/DISCOVERY.md](DISCOVERY.md) for the full announce contract, including signed announces.

## Receiver Info

`GET /api/v1/server/info` (or the receiver's own info when queried directly) returns the canonical receiver description:

```json
{
  "service": "michi-stream-standard",
  "name": "Living Room Receiver",
  "version": "1.0.0",
  "firmware": "1.2.3",
  "api_version": "v1-lite",
  "roles": ["audio_receiver"],
  "auth": {
    "required": true,
    "strategy": "RECEIVER_BUTTON",
    "token_refresh": false
  },
  "audio": {
    "codecs": ["pcm_s16le"],
    "max_sample_rate": 48000,
    "max_channels": 2
  },
  "features": {
    "session": true,
    "volume": true,
    "heartbeat": true
  }
}
```

Hi-Fi receivers advertise `"codecs": ["pcm_s16le", "pcm_s24le"]` and their maximum sample rate (e.g. 96000).

## Endpoints

The server consumes the receiver's API. All paths are under `/api/v1/receiver-lite/`; requests use the device bearer token obtained during pairing.

### Pairing

Receivers pair through the canonical flow: `POST /api/v1/pair/start`, `GET /api/v1/pair/status`, `POST /api/v1/pair/confirm`, with auth strategy `RECEIVER_BUTTON` (pressing the physical button on the device). Session limits apply: 5 minutes max, 5 attempts, single use. See [docs/PAIRING.md](PAIRING.md).

### `POST /api/v1/receiver-lite/session`

Creates a lightweight playback session on the receiver.

```json
{
  "deviceId": "uuid-del-receptor",
  "trackId": "uuid-del-track",
  "startPlaying": true
}
```

Fields: `deviceId` (required), `trackId`, `trackIds`, `playlistId`, `startPlaying` (default `true`), `syncGroup`.

**Response `201 Created`:**

```json
{
  "sessionId": "uuid-de-la-sesion",
  "status": "active"
}
```

### `DELETE /api/v1/receiver-lite/session`

Ends the active session. **Response `204 No Content`.**

### `POST /api/v1/receiver-lite/heartbeat`

Sent every **10 seconds** to keep the session alive and report status.

```json
{
  "sessionId": "uuid-de-la-sesion",
  "state": "playing",
  "positionMs": 78000
}
```

Fields: `sessionId` and `state` (required), `positionMs`, `bufferLevel`, `volume`, `latencyMs`, `timestamp`.

**Response `200 OK`:**

```json
{
  "status": "ok",
  "serverTime": "2026-06-29T12:00:00Z"
}
```

If no heartbeat arrives for 30 seconds, the receiver is considered disconnected.

### `PUT /api/v1/receiver-lite/volume`

Sets the receiver volume.

```json
{
  "sessionId": "uuid-de-la-sesion",
  "volume": 75,
  "muted": false
}
```

**Response `200 OK`:**

```json
{
  "volume": 75,
  "muted": false
}
```

Volume range: 0–100 (integer).

### `GET /api/v1/receiver-lite/firmware`

Returns the current firmware version and update availability.

```json
{
  "currentVersion": "1.2.3",
  "latestVersion": "1.3.0",
  "updateAvailable": true,
  "releaseDate": "2026-06-01T00:00:00Z",
  "changelog": "Security fixes and performance improvements.",
  "updateUrl": "http://192.168.1.100:8600/firmware/v1.3.0.bin",
  "checksum": "sha256:a1b2c3d4...",
  "lastChecked": "2026-06-29T10:00:00Z",
  "lastUpdated": "2026-05-01T10:00:00Z"
}
```

### `POST /api/v1/receiver-lite/firmware`

Initiates a firmware update.

```json
{
  "url": "http://192.168.1.100:8600/firmware/v1.3.0.bin",
  "checksum": "sha256:a1b2c3d4..."
}
```

**Response `202 Accepted`:**

```json
{
  "status": "updating",
  "startedAt": "2026-06-29T12:00:00Z"
}
```

### `GET /api/v1/receiver-lite/config`

Returns the receiver configuration.

```json
{
  "deviceName": "Living Room Receiver",
  "audioOutput": "analog",
  "sampleRate": 48000,
  "bitDepth": 24,
  "bufferSize": 2048,
  "volumeControl": true,
  "autoSync": false
}
```

`audioOutput` is one of `hdmi`, `optical`, `analog`, `bluetooth`.

### `PUT /api/v1/receiver-lite/config`

Updates the receiver configuration.

**Response `200 OK`:**

```json
{
  "status": "applied",
  "requiresReboot": false
}
```

## Server-side Receiver Management

The server tracks receivers through:

- `GET /api/v1/receivers` — list discovered and registered receivers.
- `POST /api/v1/receivers/discover` — trigger network discovery (`timeout` query param, 1–30 s, default 5).

There is no `POST /api/v1/receivers/announce` endpoint: presence is announced exclusively through the multicast/mDNS discovery channel.

## Hardware Notes

### Standard

| Characteristic | Specification |
|----------------|---------------|
| Audio output | Stereo jack 3.5mm |
| DAC | Integrated in SoC |
| RAM | ~512 KB |
| Storage | ~4 MB |
| Max format | 16-bit / 48 kHz (`pcm_s16le`) |
| Connectivity | Wi-Fi 802.11 b/g/n |
| Typical use | Secondary rooms, bathroom |

### Hi-Fi

| Characteristic | Specification |
|----------------|---------------|
| Audio output | RCA (L/R) + jack 3.5mm |
| DAC | PCM5242 or better |
| RAM | ~4 MB |
| Storage | ~16 MB |
| Formats | Up to 24-bit / 96 kHz (`pcm_s16le`, `pcm_s24le`) |
| Connectivity | Wi-Fi 802.11 b/g/n + Ethernet |
| Typical use | Living room, main setup |

## Related

- [docs/DISCOVERY.md](DISCOVERY.md) — announce and discovery contract
- [docs/PAIRING.md](PAIRING.md) — pairing flow and limits
- [docs/MICHI_LINK_API_V1.md](MICHI_LINK_API_V1.md) — full API reference (receiver-lite section)
- `openapi/michi-link-v1.yaml` — OpenAPI source for the receiver paths
