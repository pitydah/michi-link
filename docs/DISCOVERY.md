# Discovery Protocol

Devices on the local network discover each other using two concurrent methods: **UDP multicast announce** and **mDNS**. Discovery is the entry point of the Michi Link contract: it carries no private information, and identity is only established through signed announces or the pairing flow.

## Network Contract

| Constant | Value |
|----------|-------|
| Multicast group | `224.0.0.167` (IPv4) |
| Multicast port | `53318` |
| Announce interval | 30 seconds |
| Offline timeout | 90 seconds (a peer is removed if no announce arrives within this window) |
| Encoding | JSON, UTF-8 |
| Maximum announce size | 8 KiB |
| mDNS service | `_michi-link._tcp.local` |
| Timestamp window (signed) | ±90 seconds |
| Replay cache | nonce accepted once per `michi_id` |

These constants are the single source of truth in `crates/michi-identity/src/discovery.rs` (`MULTICAST_GROUP`, `MULTICAST_PORT`, `ANNOUNCE_INTERVAL`, `OFFLINE_TIMEOUT`, `MAX_ANNOUNCE_BYTES`, `MDNS_SERVICE`, `TIMESTAMP_WINDOW_MS`).

## UDP Multicast Announce

Every 30 seconds each device broadcasts a JSON announce over UDP to `224.0.0.167:53318`.

### Announce Payload

| Field | Type | Description |
|-------|------|-------------|
| `device_id` | string | **Stable** device identifier, persisted across restarts. Never regenerated per announce. |
| `name` | string | Human-readable device name |
| `service` | string | Canonical service identifier (see enum below) |
| `roles` | string[] | Roles this device can fulfill (see enum below) |
| `api_version` | string | `"v1"` or `"v1-lite"` |
| `host` | string | IP address or hostname of the HTTP API |
| `port` | number | HTTP API port |
| `features` | object | **Boolean-only** feature flags |

### Identity Group (all-or-nothing)

A signed announce adds exactly these five fields. They must all be present or all absent:

| Field | Type | Description |
|-------|------|-------------|
| `michi_id` | string | base64url of BLAKE3(public key raw bytes), 43 chars, no padding |
| `public_key` | string | Ed25519 public key, base64url, 43 chars, no padding |
| `signature` | string | Ed25519 signature over the canonical payload, base64url, 86 chars, no padding |
| `timestamp_ms` | number | Unix epoch milliseconds at signing time |
| `nonce` | string | base64url random nonce (≥ 22 chars, 16 raw bytes) |

Capabilities are **not** part of the announce: they are negotiated via `GET /api/v1/server/info` (see below).

### Canonical Example

```json
{
  "device_id": "stable-device-01",
  "name": "Living Room Player",
  "service": "michi-music-player",
  "roles": ["desktop_player", "library_master", "sync_host"],
  "api_version": "v1",
  "host": "192.168.1.10",
  "port": 8400,
  "features": {
    "library": true,
    "events": false
  },
  "michi_id": "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
  "public_key": "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ",
  "signature": "DTlMt9BYH_TnYgKAeGd8zTpza-w5b8BDm9AyIoAW2p0clD7JrzwN9cwPY5y48K14x_0z2TPq7-LTXdNTqmhr-w",
  "timestamp_ms": 1785974884586,
  "nonce": "VFfZjzw8JeAM7-RFiTSrMA"
}
```

The canonical example lives in `examples/discovery-announce.json` and its schema in `schemas/discovery-announce.schema.json`.

### Services

| Service | Description |
|---------|-------------|
| `michi-music-player` | Desktop player |
| `michi-micro-server` | Home server |
| `michi-mobile` | Android app |
| `michi-stream-standard` | Receiver, standard (jack 3.5mm) |
| `michi-stream-hifi` | Receiver, Hi-Fi (DAC, RCA) |

### Roles

| Role | Belongs to |
|------|-----------|
| `desktop_player`, `library_master`, `sync_host` | Player |
| `music_server`, `library_host`, `playback_host` | Micro Server |
| `mobile_player`, `remote_controller`, `sync_client` | Mobile |
| `audio_receiver` | Stream receivers |

### Service profiles

Each service advertises a **fixed announce profile**: `api_version`, `roles` and `features` are invariants per service, and the announce is rejected as a `ContractViolation` when the profile is incoherent (wrong `api_version`, roles outside the service's set, empty `features`, empty `roles`, or a stream receiver whose `roles` is not exactly `["audio_receiver"]`).

| Service | api_version | roles (allowed) | Notes |
|---------|-------------|-----------------|-------|
| `michi-music-player` | `v1` | `desktop_player`, `library_master`, `sync_host` | — |
| `michi-micro-server` | `v1` | `music_server`, `library_host`, `playback_host` | — |
| `michi-mobile` | `v1` | `mobile_player`, `remote_controller`, `sync_client` | — |
| `michi-stream-standard` | `v1-lite` | exactly `["audio_receiver"]` | receivers announce `v1-lite`, never `v1` |
| `michi-stream-hifi` | `v1-lite` | exactly `["audio_receiver"]` | receivers announce `v1-lite`, never `v1` |

`features` is a **boolean-only** map and is never empty. In the Rust reference implementation the profile is represented by `AnnounceProfile` (`crates/michi-identity/src/types.rs`), validated by `DiscoveryEngine::validate_profile` before signing:

```rust,ignore
use michi_identity::types::{
    AnnounceProfile, ApiVersion, Role, Service,
};

let profile = AnnounceProfile {
    device_id: "stable-device-01".into(),
    name: "Living Room Player".into(),
    service: Service::MusicPlayer,
    roles: vec![Role::DesktopPlayer, Role::LibraryMaster, Role::SyncHost],
    api_version: ApiVersion::V1,
    host: "192.168.1.10".into(),
    port: 8400,
    features: [("library".to_string(), true)].into_iter().collect(),
};

DiscoveryEngine::validate_profile(&profile)?;
```

Example announce for a player (full `v1` profile, signed):

```json
{
  "device_id": "stable-device-01",
  "name": "Living Room Player",
  "service": "michi-music-player",
  "roles": ["desktop_player", "library_master", "sync_host"],
  "api_version": "v1",
  "host": "192.168.1.10",
  "port": 8400,
  "features": { "library": true, "events": false }
}
```

Example announce for a Hi-Fi stream receiver (`v1-lite`, exactly one role):

```json
{
  "device_id": "rec-hifi-01",
  "name": "Michi Music Stream Hi-Fi",
  "service": "michi-stream-hifi",
  "roles": ["audio_receiver"],
  "api_version": "v1-lite",
  "host": "192.168.1.20",
  "port": 8410,
  "features": { "session": true, "volume": true, "heartbeat": true }
}
```

## Signed Announce Verification

The signature covers **every functional field** (`device_id`, `name`, `service`, `roles`, `api_version`, `host`, `port`, `features`, `michi_id`, `public_key`, `timestamp_ms`, `nonce`) using a **deterministic canonical serialization**: a JSON object with keys in lexicographic order. The same logical announce always produces the same bytes.

Verification pipeline (in order):

1. **Signature** — verify Ed25519 over the canonical payload with the announced `public_key`. Invalid → `Invalid`.
2. **Identity coherence** — `michi_id` must equal `base64url(BLAKE3(public_key raw bytes))` derived from the announced key. Mismatch → `Invalid`.
3. **Freshness** — `timestamp_ms` must be within ±90 s of the receiver clock. Out of window → error (`TimestampOutOfWindow`).
4. **Replay protection** — the `(michi_id, nonce)` pair must not have been seen before within the window. Replay → error (`ReplayDetected`).
5. **Host coherence** — when the datagram source is known and the announced `host` is an explicit IP, it must match the source IP. Mismatch → `Invalid`.
6. **Partial group** — if only some identity fields are present, the announce is **invalid**, never partially trusted.
7. **Unsigned** — no identity group at all: the announce is accepted as **`Untrusted`** (legacy interoperability) but is never treated as verified identity.

Trust levels: `Verified(michi_id)` — valid signature and coherent identity; `Untrusted(device_id)` — unsigned legacy announce; `Invalid` — present but broken.

## mDNS

Devices also advertise via multicast DNS using the service type:

```
_michi-link._tcp.local
```

### Minimal TXT Records

The TXT records are a **minimum set** for coarse filtering; capabilities must NOT be expanded in TXT records:

| Key | Example Value |
|-----|---------------|
| `device_id` | `stable-device-01` |
| `service` | `michi-music-player` |
| `api_version` | `v1` |
| `roles` | `desktop_player,library_master,sync_host` |
| `auth_strategy` | `SERVER_CODE` |

Capabilities are **only** advertised through `GET /api/v1/server/info` (`features` object), never through TXT records. Signed identity material is only carried by UDP announces, not by mDNS TXT records.

## server/info

Once a device is discovered, its capabilities and identity are fetched from `GET /api/v1/server/info`:

- Required fields: `service`, `name`, `version`, `api_version`, `roles`, `auth`, `features`.
- `auth.required` is `true` for all servers.
- `features` values are boolean-only.
- `events` stays `false` until certified (see `docs/BETA_GATE.md`).
- No undeclared capabilities (e.g. rooms, transcoding) may be advertised without implementation.
- No internal paths, secrets, or extra version fields are allowed.

## Offline Handling

1. Join UDP multicast group `224.0.0.167` on port `53318`.
2. Listen for JSON payloads (max 8 KiB).
3. Parse and register the device by `device_id`.
4. Reset the 90-second offline timer on each received announce.
5. Remove the device from the registry if the timer expires.

## Reference

- Implementation and tests: `crates/michi-identity/src/discovery.rs`
- Schema: `schemas/discovery-announce.schema.json`
- Example: `examples/discovery-announce.json`, `examples/michi-identity-announce-signed.json`
- OpenAPI: `openapi/michi-link-v1.yaml`
- Identity foundation: [docs/MICHI_IDENTITY.md](MICHI_IDENTITY.md)
