# Security Policy

## Reporting Vulnerabilities

Michi Link is a wire-format contract; security issues live in the protocol design, in the reference implementation (`crates/michi-identity`), or in the schemas. If you find a vulnerability, report it privately before disclosing it publicly (responsible disclosure).

Please report issues by opening a private security advisory on the repository (GitHub Security Advisories) or by contacting the maintainers directly. Do not open a public issue for an active vulnerability.

**What to include:**

- Affected component (identity, pairing, discovery, receivers, contract schema).
- Severity estimate and attack scenario (who can exploit it, from where).
- Reproduction steps or a minimal example.
- Suggested fix, if known.

We aim to acknowledge reports within 5 business days and to ship a fix before public disclosure.

## Security Scope

The following areas are in scope:

- **Contract:** `schemas/`, `openapi/michi-link-v1.yaml`, canonical error codes.
- **Identity:** key generation, persistence, derivation of `michi_id`, signature verification.
- **Pairing:** PIN flow, session limits, QR URIs, token issuance.
- **Discovery:** UDP multicast announces, mDNS records, signed announce verification.
- **Receivers:** v1-lite protocol, firmware updates, receiver authentication.

Out of scope: implementation-specific bugs in consumers (Player, Micro Server, Mobile, Stream) — report those to their respective repositories.

## Secrets

- The device private key is stored **encrypted at rest** with ChaCha20-Poly1305 (AEAD) in the identity file. The encryption key is derived as `blake3(context || salt || password)`, with the format context and identity scheme bound as explicit AAD.
- The identity file is written atomically (temp file + rename) with **0600 permissions** on Unix.
- The password is **local to the device** and never transmitted over the network.
- **Never transmit secrets** (private keys, passwords, pairing secrets) over the wire. The pairing PIN is verified through a keyed verifier, never sent as plaintext hash material that could be replayed.

## Identity

- Scheme: `ed25519-blake3-v1`. `michi_id = base64url(BLAKE3(public_key raw 32 bytes))` — 43 characters. Hex is not a canonical representation.
- Signatures are Ed25519 over a **deterministic canonical serialization** (JSON keys in lexicographic order) of every functional field.
- Timestamp freshness window: **±90 seconds**.
- Replay protection: nonces are accepted **once per identity**.
- Unsigned announces are classified `Untrusted` and never treated as verified identity.

## Pairing

- Sessions last **5 minutes max**, allow **5 attempts max**, and are **single use** (consumed after success).
- The PIN verifier is keyed with a server-side random secret (`blake3(server_secret || pin)`), so a 6-digit PIN **cannot be brute-forced offline** from anything exposed to the client.
- PIN comparison is **constant-time**.
- `pin_hash` is never exposed. The QR URI contains no secrets, no final tokens, and no PIN hash.

## Discovery

- Canonical group: **UDP multicast `224.0.0.167:53318`**. mDNS: `_michi-link._tcp.local`.
- Announces are JSON UTF-8, max 8 KiB, sent every 30 seconds; a peer is offline after 90 seconds.
- Signed announces carry the full identity group (`michi_id`, `public_key`, `signature`, `timestamp_ms`, `nonce`) — all-or-nothing. A partially signed announce is invalid.
- An announced explicit IP host must match the datagram source (host coherence).
- Consumers must never treat unsigned announces as verified identity.

## Key Rotation

- Rotation (regeneration) is **explicit and user-initiated only**: delete the identity file and regenerate. The library never rotates silently.
- Migration from the legacy v1 format (XOR + hostname wrap) to v2 (AEAD) is **automatic** and preserves the same identity — it is a format upgrade, not a rotation.

## Corrupted Files

- A corrupted or unauthenticated identity file returns an **explicit error** and is never overwritten or silently regenerated.
- **Recovery procedure:** restore the identity file from a backup. If no backup exists, regeneration must be an explicit operator decision (deleting the file and recreating the identity invalidates previously issued trust).

## Supported Versions

Only the contract versions `v1` and `v1-lite` are supported. Anything else must be treated as unsupported.

| Contract version | Status |
|------------------|--------|
| `v1` | Supported (active contract) |
| `v1-lite` | Supported (receivers) |
| Others | Unsupported — must not be advertised |

## Network Model Limitation

The protocol assumes a **trusted local network**. There is **no TLS requirement** for the local network transport in the current contract.

- On a hostile network, announces, pairing traffic, and API calls can be observed or tampered with.
- **Recommended mitigations** for deployments on untrusted networks:
  - Enable TLS (HTTPS) on the HTTP API wherever possible.
  - Prefer signed announces and reject unsigned peers when the security posture requires it.
  - Keep pairing short-lived and single-use.
  - Do not expose the pairing endpoints to the internet.
  - Firewall the discovery group and API ports to trusted subnets.
