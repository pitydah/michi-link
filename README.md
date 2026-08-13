# Michi Link

**The universal protocol for the Michi ecosystem**

Michi Link is the official protocol specification that defines how every component of the Michi ecosystem communicates. This repository is **not** an application or library — it is the contract that all implementations must follow.

The Michi ecosystem consists of:

- **Michi Music Player** — Desktop music player for Linux/KDE (Python, PySide6, GStreamer). Master center for library management, metadata, artwork, lyrics, playlists, audio profiles, and synchronization.
- **Michi Micro Server** — Lightweight home server (Rust/Tokio/Axum/SQLite). Receives music from Player, stores library backup, serves music to Mobile, reproduces autonomously, and distributes audio across the house.
- **Michi Music Mobile** — Android app (Kotlin/Jetpack Compose/Media3). Local/offline playback, download/stream from Micro Server, and remote control for the ecosystem.
- **Michi Music Stream** — Family of physical audio receivers. Standard (jack 3.5mm) and Hi-Fi (DAC, RCA stereo). Only receives audio and converts to physical output.
- **Michi Big Server** — Future high-capacity server for large libraries. **Out of scope of the active contract** until a formal project exists; it may return as a future section.

---

## What is Michi Link?

Michi Link is a **contract** — a wire-format specification that any Michi component can speak. It decouples producers from consumers so that a player, server, mobile app, or stream receiver can discover, pair, and interoperate without shared code or tight coupling.

**Michi Link comes integrated in each Michi app.** End users never install it separately.

---

## Core Concepts

| Concept | Description |
|---------|-------------|
| **Discovery** | UDP multicast announce and/or mDNS for service discovery on the local network |
| **Pairing** | Trusted handshake between devices with PIN + identity exchange |
| **Permissions** | Device-scoped authorization tokens for fine-grained access control |
| **Library** | Query, browse, and search music metadata across devices |
| **Streaming** | Audio transport via HTTP Range requests |
| **Sync** | Manifest-based incremental synchronization for offline devices |
| **Playback Control** | Play, pause, seek, volume, and queue management commands |
| **Audio Chains** | Modular pipeline: Source + Controller + Output + Profile |
| **Receivers** | Physical audio output devices (v1-lite protocol for constrained hardware) |
| **Rooms** | Groups of receivers for synchronized multi-room audio |
| **Events** | Real-time push notifications via WebSocket |

---

## Transport Architecture

| Layer | Protocol | Purpose |
|-------|----------|---------|
| **Discovery** | UDP multicast (`224.0.0.167:53318`) / mDNS (`_michi-link._tcp.local`) | Device announcement and discovery |
| **Primary API** | HTTP REST (`/api/v1`) | All data operations: library, sync, streaming, playback, queue, rooms |
| **Real-time** | WebSocket (`/api/v1/events`) | Event notifications (state changes, device events) |
| **Streaming** | HTTP with Range support | Audio streaming and download |

---

## Contract

The canonical contract is defined in `schemas/` (JSON Schema draft-07), `openapi/michi-link-v1.yaml` (OpenAPI 3.0) and `crates/michi-identity/` (reference implementation of identity, pairing and discovery primitives).

- **API version:** `api_version` is an enum — `"v1"` (full contract) or `"v1-lite"` (constrained devices). Never a semantic version. Version is tracked in three independent dimensions: `api_version` (contract), `version` (application), `firmware` (receiver).
- **Identity:** scheme `ed25519-blake3-v1`. `michi_id` is the BLAKE3 hash of the raw Ed25519 public key, encoded base64url (43 chars). At-rest encryption uses **Argon2id** (64 MiB, t=3, p=1) + ChaCha20-Poly1305 with the full canonical header as AAD; legacy v1/v2 files migrate automatically to format v3. See [docs/MICHI_IDENTITY.md](docs/MICHI_IDENTITY.md).
- **Converged contract:** the wire format is strict base64url without padding (`michi_id`/`public_key` exactly 43 chars, signatures 86, nonces ≥ 22; `+`, `/`, `=` forbidden), and pairing is a single unified session + PIN + Ed25519 challenge flow. See [docs/PAIRING.md](docs/PAIRING.md) and [docs/DISCOVERY.md](docs/DISCOVERY.md).
- **Auth:** bearer tokens obtained through the pairing flow; strategies `PLAYER_PASSWORD`, `SERVER_CODE`, `ED25519_CHALLENGE`, `RECEIVER_BUTTON`, `LEGACY`.
- **Errors:** canonical envelope `{"error": {"code", "message", "details"?, "request_id"?}}` with 20 error codes. See [docs/MICHI_LINK_API_V1.md](docs/MICHI_LINK_API_V1.md).

---

## Repository Structure

| Path | Contents |
|------|----------|
| `docs/` | Specification documents for every subsystem |
| `openapi/` | OpenAPI 3.0 specification |
| `schemas/` | JSON Schema (draft-07) for every entity |
| `examples/` | Annotated JSON examples for every flow |
| `contracts/receiver-v1-lite/` | Versioned receiver conformance bundle: OpenAPI, schemas, examples, vectors, `VERSION`, `UPSTREAM_COMMIT` and an ordered SHA-256 `manifest.json`. Byte-for-byte reproducible; the single contractual input for downstream receivers |
| `tests/contract/` | Conformance tests validating examples against schemas (248 checks) |
| `tests/identity_contract/` | Identity contract tests — michi_id, signatures, announcements (22 checks) |
| `tests/cross_layer/` | Cross-layer checks: schemas ↔ OpenAPI ↔ crate types (84 checks) |
| `crates/michi-identity/` | Reference implementation: identity, pairing, discovery, QR (Rust, 88 tests) |
| `scripts/` | Repository tooling (contract policy scanner, deterministic bundle builder) |

---

## Quick Start for Integrators

```text
1. Discover services via UDP multicast (224.0.0.167:53318) or mDNS (_michi-link._tcp.local).
2. Query server identity: GET /api/v1/server/info
3. Initiate pairing:      POST /api/v1/pair/start
4. Confirm pairing:       POST /api/v1/pair/confirm
5. Use Authorization: Bearer <device_token> on all subsequent requests.
6. Use REST /api/v1 for library, sync, streaming, playback, queue, and rooms.
7. Use WebSocket /api/v1/events only for real-time event notifications.
```

---

## Validation

The repository ships a contract test suite and a policy scanner. Run them before merging changes:

```bash
# JSON contract conformance (examples vs schemas) — 248 checks
cd tests/contract && npm test && cd ../..

# Identity contract (michi_id, signatures, discovery announcements) — 22 checks
cd tests/identity_contract && npm test && cd ../..

# Cross-layer checks (schemas ↔ OpenAPI ↔ crate types) — 84 checks
cd tests/cross_layer && npm test && cd ../..

# Reference implementation — 88 tests, clippy 0 warnings
cd crates/michi-identity && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-targets --all-features && cd ../..

# OpenAPI lint
npx --yes @redocly/cli@2 lint openapi/michi-link-v1.yaml --extends=minimal

# Retired-token policy scan (must exit 0)
python3 scripts/contract-policy.py

# Receiver v1-lite bundle reproducibility (CI job: bundle-reproducibility)
cd crates/michi-identity && cargo run -q --example generate_contract_vectors -- ../../tests/vectors/generated --receiver ../../tests/vectors/receiver-v1-lite && cd ../..
python3 scripts/build-receiver-bundle.py
git diff --exit-code -- contracts/receiver-v1-lite tests/vectors/generated tests/vectors/receiver-v1-lite
```

---

## State

The contract is **defined and tested**: schemas, OpenAPI, examples, identity tests and the reference crate are in place. The receiver v1-lite profile is frozen (ADR-0001) and published in-tree as a versioned, byte-for-byte reproducible conformance bundle at `contracts/receiver-v1-lite/` (`1.0.0-alpha.1`). Downstream implementations vendor the bundle without modification and verify it against the ordered SHA-256 manifest; see [docs/DOWNSTREAM_MIGRATION.md](docs/DOWNSTREAM_MIGRATION.md). The ecosystem beta is **globally CLOSED** until the scenarios in [docs/BETA_GATE.md](docs/BETA_GATE.md) reach `NETWORK_E2E_PASS` evidence. Evidence levels are defined in [docs/TEST_LEVELS.md](docs/TEST_LEVELS.md) — no feature may be claimed as done without evidence, and no hardware is certified: receiver interoperability with physical hardware and the Stream simulator remains NOT_TESTED.

---

## Versioning

Versioning is split into three independent dimensions:

| Dimension | Field | Meaning |
|-----------|-------|---------|
| Contract | `api_version` | `"v1"` or `"v1-lite"`. Breaking changes require a deliberate major contract change. |
| Application | `version` | Version of the implementation (application-specific). |
| Receiver firmware | `firmware` | Version of the firmware on physical receivers. |

Michi Link v1-lite is a subset of v1 for resource-constrained devices (receivers), not a different version. There is no separate link-level version field.

---

## License

Licensed under the [MIT License](LICENSE).

---

## Build Status

[![Spec Status](https://img.shields.io/badge/spec-v1-blue.svg)](https://github.com/pitydah/michi-link)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
