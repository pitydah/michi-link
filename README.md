# Michi Link API v1

**The universal protocol for the Michi ecosystem**

Michi Link is the official protocol specification that defines how every component of the Michi ecosystem communicates. This repository is **not** an application or library — it is the contract that all implementations must follow.

The Michi ecosystem consists of:

- **Michi Music Player** — Desktop music player for Linux/KDE (Python, PySide6, GStreamer). Master center for library management, metadata, artwork, lyrics, playlists, audio profiles, and synchronization.
- **Michi Micro Server** — Lightweight home server (Rust/Tokio/Axum/SQLite). Receives music from Player, stores library backup, serves music to Mobile, reproduces autonomously, and distributes audio across the house.
- **Michi Music Mobile** — Android app (Kotlin/Jetpack Compose/Media3). Local/offline playback, download/stream from Micro Server, and remote control for the ecosystem.
- **Michi Music Stream** — Family of physical audio receivers. Standard (jack 3.5mm) and Hi-Fi (DAC, RCA stereo). Only receives audio and converts to physical output.
- **Michi Big Server** — Future high-capacity server for large libraries.

---

## What is Michi Link?

Michi Link is a **contract** — a wire-format specification that any Michi component can speak. It decouples producers from consumers so that a player, server, mobile app, or stream receiver can discover, pair, and interoperate without shared code or tight coupling.

**Michi Link comes integrated in each Michi app.** End users never install it separately.

---

## Core Concepts

| Concept | Description |
|---------|-------------|
| **Discovery** | UDP multicast announce and/or mDNS for service discovery on the local network |
| **Pairing** | Trusted handshake between devices with token exchange |
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
| **Discovery** | UDP multicast (port 42069) / mDNS | Device announcement and discovery |
| **Primary API** | HTTP REST (`/api/v1`) | All data operations: library, sync, streaming, playback, queue, rooms |
| **Real-time** | WebSocket (`/api/v1/events`) | Event notifications (state changes, device events) |
| **Streaming** | HTTP with Range support | Audio streaming and download |

---

## Repository Structure

| Path | Contents |
|------|----------|
| `docs/` | Specification documents for every subsystem |
| `openapi/` | OpenAPI 3.0 specification |
| `schemas/` | JSON Schema (draft-07) for every entity |
| `examples/` | Annotated JSON examples for every flow |
| `tests/contract/` | Conformance tests validating examples against schemas |

---

## Quick Start for Integrators

```text
1. Discover services via UDP multicast (port 42069) or mDNS (_michi-link._tcp).
2. Query server identity: GET /api/v1/server/info
3. Initiate pairing:  POST /api/v1/pair/start
4. Confirm pairing:   POST /api/v1/pair/confirm
5. Use Authorization: Bearer <device_token> on all subsequent requests.
6. Use REST /api/v1 for library, sync, streaming, playback, queue, and rooms.
7. Use WebSocket /api/v1/events only for real-time event notifications.
```

---

## Versioning

This specification uses **semantic versioning** (`major.minor.patch`). The current major version is **v1**. Breaking changes increment the major version. Backward compatibility is guaranteed within a major version.

Michi Link v1-lite is a subset of v1 for resource-constrained devices (receivers), not a different version.

---

## License

Licensed under the [MIT License](LICENSE).

---

## Build Status

[![Spec Status](https://img.shields.io/badge/spec-v1-blue.svg)](https://github.com/pitydah/michi-link)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
