# Downstream Migration Guide

This document is the migration matrix for the four consumer repositories — **Player**, **Micro Server**, **Mobile**, and **Stream** — from their legacy contract usage to the canonical contract defined in this repository (schemas, OpenAPI, identity crate). It documents what each consumer must change; the changes themselves happen in their own repositories.

> The canonical contract is the source of truth: `schemas/`, `openapi/michi-link-v1.yaml`, and `crates/michi-identity` (reference implementation).

## Common to all consumers

| Change | Old (retired) | New (canonical) | Automatic? |
|--------|---------------|-----------------|------------|
| Version field | `michi_link_version` (semantic version) | `api_version` enum: `"v1"` / `"v1-lite"` | Manual | <!-- michi-policy:exclude -->
| Service names | `michi-player`, `michi-server`, `michi-stream` aliases | `michi-music-player`, `michi-micro-server`, `michi-mobile`, `michi-stream-standard`, `michi-stream-hifi` | Manual | <!-- michi-policy:exclude -->
| Discovery endpoint | UDP `255.255.255.255:42069` | UDP multicast `224.0.0.167:53318` | Manual | <!-- michi-policy:exclude -->
| mDNS | `_michi-link._tcp` (or none) | `_michi-link._tcp.local` | Manual |
| Identity | SHA-256/hex, ad-hoc key formats | `ed25519-blake3-v1`, `michi_id = base64url(BLAKE3(pk))` (43 chars), base64/base64url | Mixed (see per repo) |
| Error envelope | Ad-hoc error bodies | `{"error": {"code", "message", "details"?, "request_id"?}}` + 17 canonical codes | Manual |

The full 17 error codes: `INVALID_REQUEST`, `UNAUTHORIZED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `RATE_LIMITED`, `INTERNAL_ERROR`, `NOT_IMPLEMENTED`, `PAIRING_EXPIRED`, `PAIRING_ATTEMPTS_EXCEEDED`, `PAIRING_KEY_MISMATCH`, `IDENTITY_CORRUPTED`, `SIGNATURE_INVALID`, `REPLAY_DETECTED`, `IDEMPOTENCY_KEY_REUSE`, `TRACK_NOT_FOUND`, `IMPORT_SESSION_EXPIRED`.

## Michi Music Player

| Change | Detail |
|--------|--------|
| Drop `michi_link_version` | Advertise `api_version: "v1"` in announces and `server/info` | <!-- michi-policy:exclude -->
| Discovery | Join `224.0.0.167:53318`; mDNS `_michi-link._tcp.local`; parse canonical announces (stable `device_id`, boolean `features`) |
| Service name | `michi-music-player` (not `michi-player`) | <!-- michi-policy:exclude -->
| Roles | `desktop_player`, `library_master`, `sync_host` |
| Identity | Adopt `ed25519-blake3-v1`; `michi_id` base64url; verify signed announces (7-step pipeline) |
| Auth | `PLAYER_PASSWORD`; `auth.required: true` in `server/info` |
| Pairing | Canonical `/pair/start` → `/pair/status` → `/pair/confirm`; opaque bearer tokens |

## Michi Micro Server

| Change | Detail | Automatic? |
|--------|--------|------------|
| Identity storage | Migrate SHA-256/hex artifacts to the identity crate (v0.2.0+): password-protected AEAD store, `michi_id` base64url | Identity file migration v1→v2 is **automatic** in the crate; code adoption is manual |
| Discovery | Canonical multicast group + port; **fully signed announces** (signature over all functional fields, `timestamp_ms`, `nonce`, ±90 s window, replay cache, host coherence) | Manual |
| Pairing | 5 min / 5 attempts / single use; canonical errors `PAIRING_EXPIRED`, `PAIRING_ATTEMPTS_EXCEEDED`, `PAIRING_KEY_MISMATCH`; keyed PIN verifier (never expose `pin_hash`) | Manual |
| Errors | Adopt the canonical 17-code envelope | Manual |
| Retired fields | Remove `michi_link_version`, `server_name`, `server_version`, `capabilities` from `server/info` | Manual | <!-- michi-policy:exclude -->
| Service name | `michi-micro-server` (not `michi-server`) | Manual | <!-- michi-policy:exclude -->
| Roles | `music_server`, `library_host`, `playback_host` | Manual |

## Michi Mobile

| Change | Detail |
|--------|--------|
| Drop `michiLinkVersion` | Remove from the DTO; validate `api_version` (`"v1"`/`"v1-lite"`) from `server/info` |
| Trust storage | Store the server's `michi_id` as the trust fingerprint; verify identity on every connection and detect key changes |
| Discovery | Parse canonical announces; treat unsigned announces as `Untrusted` |
| New errors to handle | `PAIRING_*`, `SIGNATURE_INVALID`, `REPLAY_DETECTED`, `IDENTITY_CORRUPTED` |
| Service name | `michi-mobile` |
| Roles | `mobile_player`, `remote_controller`, `sync_client` |

## Michi Music Stream

| Change | Detail |
|--------|--------|
| Drop `michi_link_version` | Advertise `api_version: "v1-lite"` | <!-- michi-policy:exclude -->
| Codecs | Limit to `pcm_s16le` (standard) and `pcm_s16le` + `pcm_s24le` (hi-fi). Remove any other codec (e.g. opus) from advertised capabilities |
| Identity & pairing | Receiver identity per `ed25519-blake3-v1`; pairing with `RECEIVER_BUTTON`, `token_refresh: false`, via canonical `/pair/start` → `/pair/status` → `/pair/confirm` |
| Endpoints | Serve `/api/v1/receiver-lite/{session,heartbeat,volume,firmware,config}`; presence via discovery (no announce endpoint) |
| Heartbeat | Every 10 seconds |
| Simulator & firmware | Align the simulator and firmware images to the same receiver contract |
| Service name | `michi-stream-standard` / `michi-stream-hifi` (not `michi-stream`) | <!-- michi-policy:exclude -->
| Roles | `["audio_receiver"]` |

## Breaking-change matrix

| Repo | Breaking | Automatic | Effort |
|------|----------|-----------|--------|
| Player | Discovery port/group, `api_version`, service name, error handling | — | Medium |
| Micro Server | Discovery signed announces, pairing limits, error envelope, identity store | Identity file migration (v1→v2) is automatic in the crate | High |
| Mobile | DTO field removal, `api_version` validation, error handling, trust fingerprint | — | Medium |
| Stream | `api_version`, codecs, endpoints, heartbeat interval, pairing strategy | — | High |

## Reference material

- Contract: `schemas/`, `openapi/michi-link-v1.yaml`, `examples/`
- Identity, pairing, discovery, QR: `crates/michi-identity/`
- Identity contract tests: `tests/identity_contract/`
- Evidence levels: [docs/TEST_LEVELS.md](TEST_LEVELS.md)
- Beta readiness: [docs/BETA_GATE.md](BETA_GATE.md)
