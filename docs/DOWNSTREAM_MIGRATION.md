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
| Identity | SHA-256/hex, ad-hoc key formats | `ed25519-blake3-v1`, `michi_id = base64url(BLAKE3(pk))` (43 chars), base64url | Mixed (see per repo) |
| Wire encoding | Hex / standard base64 / padded values | **base64url strict**: 43 chars for `michi_id`/`public_key`, 86 for signatures, ≥ 22 for nonces; `+`, `/`, `=` forbidden on the wire | Manual (validation in schema patterns) |
| Error envelope | Ad-hoc error bodies | `{"error": {"code", "message", "details"?, "request_id"?}}` + 20 canonical codes | Manual |
| Pairing flow | Code-based / proof-based pairing | Unified session + PIN + Ed25519 challenge (`/pair/start` with `challenge_nonce`/`challenge_signature` → session → `/pair/confirm` with `pin`) | Manual |
| Pairing DTOs | Legacy `pairing_code`, `pin_proof`, `pin_proof_signature`, camelCase fields | `schemas/pair-start.schema.json`, `pair-start-response`, `pair-confirm`, `pair-confirm-response`; snake_case only | Manual | <!-- michi-policy:exclude -->
| Pairing errors | `PAIRING_EXPIRED` / `PAIRING_ATTEMPTS_EXCEEDED` / `PAIRING_KEY_MISMATCH` | + `PAIRING_NOT_FOUND`, `PAIRING_ALREADY_CONSUMED`, `PAIRING_PIN_MISMATCH`, `RATE_LIMITED` | Manual |
| Track/Artwork paths | `path`/`file_path`/`absolute_path`/`local_path`/`mount_path` on DTOs | Removed — media is addressed by endpoint URLs only | Manual |
| Identity file KDF | XOR wrap (v1), `blake3`-direct (v2) | **Argon2id** (64 MiB, t=3, p=1, 0x13) + ChaCha20-Poly1305, AAD = full canonical header | **Automatic** in the crate (v1→v3 and v2→v3 on load) |
| Identity file format | `format_version` 1 / 2 | `format_version` **3** | **Automatic** in the crate (v1→v3 and v2→v3 on load) |
| Announce profiles | Ad-hoc roles per service | Fixed per-service invariants: player/micro/mobile → `v1`; streams → `v1-lite` with exactly `["audio_receiver"]`; incoherent profiles rejected (`ContractViolation`) | Manual |
| Receiver audio profiles | Ad-hoc codec lists | `michi-stream-standard`: `pcm_s16le`, ≤ 96000 Hz, 2 ch; `michi-stream-hifi`: `pcm_s16le` (+ optional `pcm_s24le`), ≤ 192000 Hz, 2 ch. No lossy codecs (opus forbidden) | Manual |

The full 20 error codes: `INVALID_REQUEST`, `UNAUTHORIZED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `RATE_LIMITED`, `INTERNAL_ERROR`, `NOT_IMPLEMENTED`, `PAIRING_EXPIRED`, `PAIRING_ATTEMPTS_EXCEEDED`, `PAIRING_KEY_MISMATCH`, `IDENTITY_CORRUPTED`, `SIGNATURE_INVALID`, `REPLAY_DETECTED`, `IDEMPOTENCY_KEY_REUSE`, `TRACK_NOT_FOUND`, `IMPORT_SESSION_EXPIRED`, `PAIRING_NOT_FOUND`, `PAIRING_ALREADY_CONSUMED`, `PAIRING_PIN_MISMATCH`.

## Michi Music Player

| Change | Detail |
|--------|--------|
| Drop `michi_link_version` | Advertise `api_version: "v1"` in announces and `server/info` | <!-- michi-policy:exclude -->
| Discovery | Join `224.0.0.167:53318`; mDNS `_michi-link._tcp.local`; parse canonical announces (stable `device_id`, boolean `features`, per-service profile: `v1` + `desktop_player`/`library_master`/`sync_host`) |
| Service name | `michi-music-player` (not `michi-player`) | <!-- michi-policy:exclude -->
| Roles | `desktop_player`, `library_master`, `sync_host` |
| Identity | Adopt `ed25519-blake3-v1`; `michi_id` base64url (43 chars); verify signed announces (7-step pipeline) |
| Auth | `PLAYER_PASSWORD`; `auth.required: true` in `server/info` |
| Pairing | Canonical session + PIN + challenge flow (`/pair/start` → `/pair/confirm`); opaque bearer tokens |

## Michi Micro Server

| Change | Detail | Automatic? |
|--------|--------|------------|
| Identity storage | Migrate SHA-256/hex artifacts to the identity crate (v0.2.0+): password-protected AEAD store (Argon2id + ChaCha20-Poly1305), `michi_id` base64url | Identity file migration v1→v3 and v2→v3 is **automatic** in the crate; code adoption is manual |
| Discovery | Canonical multicast group + port; **fully signed announces** (signature over all functional fields, `timestamp_ms`, `nonce`, ±90 s window, replay cache, host coherence); per-service announce profiles (`v1`, roles `music_server`/`library_host`/`playback_host`) | Manual |
| Pairing | Canonical session + PIN + challenge flow: 5 min / 5 attempts / single use / registry limits (1024 global, 8 per source, 4 per identity, 20 starts/min); canonical errors `PAIRING_NOT_FOUND`, `PAIRING_EXPIRED`, `PAIRING_ATTEMPTS_EXCEEDED`, `PAIRING_ALREADY_CONSUMED`, `PAIRING_KEY_MISMATCH`, `PAIRING_PIN_MISMATCH`, `RATE_LIMITED`; keyed PIN verifier (never expose the verifier) | Manual |
| Errors | Adopt the canonical 20-code envelope | Manual |
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
| Codecs | Receiver audio profile: standard = `pcm_s16le` only, ≤ 96000 Hz, 2 ch; hi-fi = `pcm_s16le` (+ optional `pcm_s24le`), ≤ 192000 Hz, 2 ch. Remove any other codec (e.g. opus) from advertised capabilities |
| Identity & pairing | Receiver identity per `ed25519-blake3-v1`; announce profile `v1-lite` with exactly `["audio_receiver"]`; pairing with `RECEIVER_BUTTON`, `token_refresh: false`, via canonical `/pair/start` → `/pair/confirm` |
| Endpoints | Serve `/api/v1/receiver-lite/{session,heartbeat,volume,firmware,config}`; presence via discovery (no announce endpoint) |
| Heartbeat | Every 10 seconds |
| Simulator & firmware | Align the simulator and firmware images to the same receiver contract |
| Service name | `michi-stream-standard` / `michi-stream-hifi` (not `michi-stream`) | <!-- michi-policy:exclude -->
| Roles | `["audio_receiver"]` |

## Breaking-change matrix

| Repo | Breaking | Automatic | Effort |
|------|----------|-----------|--------|
| Player | Discovery port/group, `api_version`, service name, error handling, pairing DTOs | — | Medium |
| Micro Server | Discovery signed announces, pairing limits, error envelope, identity store | Identity file migration (v1/v2→v3, Argon2id) is automatic in the crate | High |
| Mobile | DTO field removal, `api_version` validation, error handling, trust fingerprint, base64url wire | — | Medium |
| Stream | `api_version`, codecs/audio profiles, endpoints, heartbeat interval, pairing strategy | — | High |

## Reference material

- Contract: `schemas/`, `openapi/michi-link-v1.yaml`, `examples/`
- Identity, pairing, discovery, QR: `crates/michi-identity/`
- Identity contract tests: `tests/identity_contract/`
- Evidence levels: [docs/TEST_LEVELS.md](TEST_LEVELS.md)
- Beta readiness: [docs/BETA_GATE.md](BETA_GATE.md)
