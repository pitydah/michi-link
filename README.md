# Michi Link API v1

**The universal protocol for the Michi ecosystem**

Michi Link is the official protocol specification that defines how every component of the Michi ecosystem communicates. This repository is **not** an application or library — it is the contract that all implementations must follow.

The Michi ecosystem consists of:

- **Michi Music Player** — A high-performance desktop music player
- **Michi Micro Server** — A lightweight local-network server for media sharing
- **Michi Music Mobile** — Mobile remote control and playback client
- **Michi Music Stream** — Real-time streaming adapter for multi-room audio

---

## What is Michi Link?

Michi Link is a **common language** — a wire-format specification that any Michi component can speak. It decouples producers from consumers so that a player, server, mobile app, or stream adapter can discover, pair, and interoperate without shared code or tight coupling.

---

## Core Concepts

| Concept            | Description |
|--------------------|-------------|
| **Discovery**      | mDNS/DNS-SD based service announcement and lookup on the local network |
| **Pairing**        | Trusted handshake between devices using a shared secret |
| **Permissions**    | Role-based access tokens scoped to specific capabilities |
| **Library**        | Query, browse, and search media metadata across devices |
| **Streaming**      | Real-time audio transport via raw PCM, FLAC, or Opus |
| **Sync**          | Clock synchronisation for gapless multi-device playback |
| **Playback Control** | Play, pause, seek, volume, queue management commands |
| **Audio Chains**   | Modular DSP pipeline definitions (EQ, crossfade, filters) |
| **Receivers**      | Output device abstraction (HDMI, AirPlay, Bluetooth, etc.) |
| **Rooms**          | Grouping of receivers for coordinated multi-room audio |
| **Events**         | Real-time push notifications for state changes |

---

## Repository Structure

| Path | Contents |
|------|----------|
| `spec/` | The core specification documents (message schemas, sequence diagrams) |
| `spec/transport/` | WebSocket / HTTP transport layer definitions |
| `spec/payloads/` | JSON message type definitions and field semantics |
| `spec/discovery/` | mDNS service types, TXT records, and discovery flow |
| `protos/` | Optional Protocol Buffers schemas for compact binary encoding |
| `examples/` | Annotated message examples for each core flow |
| `diagrams/` | Sequence diagrams and architecture overviews (PlantUML / Mermaid) |
| `tests/` | Conformance test scenarios and expected message sequences |
| `CHANGELOG.md` | Version history and breaking-change log |

---

## How to Use This Spec

1. Read `spec/transport/connection.md` to understand the transport layer.
2. Follow the core flow in `spec/discovery/discovery.md` for initial setup.
3. Implement pairing via `spec/payloads/pairing.md`.
4. Use the message examples in `examples/` as reference during development.
5. Validate your implementation against the scenarios in `tests/`.

---

## Quick Start for Integrators

```text
1. Listen on mDNS for _michilink._tcp services.
2. Connect via WebSocket (ws://<host>:<port>/michi/v1).
3. Send a "pair" request with a generated device ID.
4. Exchange permissions tokens.
5. Query the library or start playback using the defined message types.
```

See `examples/minimal-client.md` for a complete walkthrough.

---

## Versioning

This specification uses **semantic versioning**. The current major version is **v1**. Breaking changes will increment the major version and be documented in `CHANGELOG.md`.

---

## License

Licensed under the [MIT License](LICENSE).

---

## Build Status

[![Spec Status](https://img.shields.io/badge/spec-v1-blue.svg)](https://github.com/michi/link)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
