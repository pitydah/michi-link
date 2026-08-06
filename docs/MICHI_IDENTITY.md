# Michi Identity

Decentralized identity for the Michi ecosystem. Every device holds an **Ed25519 keypair**; its public identity — the `michi_id` — is derived cryptographically from the public key, so devices recognize each other by identity instead of IPs, volatile UUIDs, or shared secrets.

## Scheme

- **Identity scheme:** `ed25519-blake3-v1`
- **Key type:** Ed25519 (32-byte seed / 32-byte public key), generated from the OS CSPRNG
- **`michi_id` derivation:** `base64url( BLAKE3( public_key_raw_32_bytes ) )` — 32 raw bytes → **43 base64url characters** (URL-safe, no padding)
- **Wire encoding (strict):** every wire value — `michi_id`, `public_key`, `signature`, `nonce` — is **base64url without padding**. `michi_id` and `public_key` are exactly 43 chars, signatures exactly 86 chars, nonces at least 22 chars. The characters `+`, `/`, `=` are **forbidden** in wire values. Hex and standard base64 are **retired from the wire**; standard base64 is only tolerated when reading legacy announces.

```
OS RNG ──→ ed25519-dalek ──→ Keypair
                                │
                                ├── secret_key → ChaCha20-Poly1305 (AEAD)
                                │      key  = Argon2id(password, salt)
                                │             (64 MiB, t=3, p=1, version 0x13)
                                │      AAD  = canonical IdentityHeader
                                │             (ALL persisted metadata)
                                │      └── identity.msgpack (format_version 3)
                                │
                                └── public_key ──→ blake3 ──→ michi_id
                                                         (base64url, 43 chars)
```

## Persistence (format v3)

The identity is persisted as MessagePack in `identity.msgpack` under the application config directory (e.g. `~/.config/michi/identity.msgpack`).

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `format_version` | uint | `3` |
| `identity_scheme` | string | `ed25519-blake3-v1` |
| `michi_id` | string | base64url, 43 chars |
| `public_key` | string | Ed25519 public key, base64url, 43 chars |
| `encrypted_secret` | string | ChaCha20-Poly1305 ciphertext of the 32-byte secret key, base64url |
| `salt` | string | 16 random bytes, base64url |
| `nonce` | string | 12 random bytes, base64url |
| `created_at` | string | RFC 3339 timestamp (preserved across migrations) |
| `device_name` | string | Human-readable device name (preserved across migrations) |

### Encryption

- **Cipher:** ChaCha20-Poly1305 (AEAD).
- **Key derivation:** **Argon2id** — memory-hard KDF, 64 MiB (`m=65536` KiB), `t=3` iterations, `p=1` lane, Argon2 version `0x13`. Parameters are persisted with the file and validated on load; out-of-range parameters are rejected as `MetadataTampered`.
- **AAD:** the **canonical header in full** — every persisted metadata field (format_version, identity_scheme, michi_id, public_key, salt, nonce, created_at, device_name) is serialized deterministically and bound as authenticated data. **Any mutation of any header field breaks authentication**, so ciphertext cannot be transplanted, re-targeted, or metadata-mangled without detection.
- **Salt/nonce:** fresh random values per write.

### Atomic writes and permissions

- The file is written via **temp file + `fsync` + rename** in the same directory, so a crash never leaves a truncated target.
- On Unix, permissions are forced to **0600**.
- On load, integrity is cross-checked: the stored `michi_id` must match the identity derived from the decrypted keypair (`IdentityMismatch` otherwise).

### Wrong password and tampering are indistinguishable by design

A wrong password and a tampered authenticated field both fail AEAD authentication and both surface as **`AuthenticationFailed`**. The crate deliberately does not distinguish them: telling them apart would give an attacker an oracle to test passwords and probe metadata. There is **no oracle** — the caller only knows "authentication failed".

Nothing is regenerated or overwritten on failure.

## Error separation

| Error | When | Code |
|-------|------|------|
| `UnsupportedFormat` | Unknown format version / scheme / KDF name this build cannot handle | `IDENTITY_CORRUPTED` |
| `MetadataTampered` | Header fields structurally invalid before decryption (bad KDF params, malformed lengths) | `IDENTITY_CORRUPTED` |
| `AuthenticationFailed` | AEAD authentication failure — covers BOTH wrong password AND tampering, by design | `IDENTITY_CORRUPTED` |
| `IdentityMismatch` | Post-decryption key/identity incoherence (defense in depth) | `IDENTITY_CORRUPTED` |

## Migration from v1 and v2 (automatic)

Both legacy formats are detected and migrated **automatically** on load to v3 (Argon2id). Migrations are format upgrades, never rotations: identity, `created_at` and `device_name` are preserved.

### v2 → v3 (BLAKE3-direct KDF)

1. The v2 file is parsed (`format_version: 2`, `encrypted_secret`, `salt`, `nonce`, `public_key`, `michi_id`).
2. The v2 key is recomputed as `blake3("michi-identity-file-v2" || salt || password)` and the secret key decrypted (ChaCha20-Poly1305, v2 AAD `context || identity_scheme`).
3. The keypair is cross-checked against the stored `michi_id` / `public_key` (mismatch → `MigrationFailed`).
4. The keypair is re-encrypted in v3 format with **Argon2id**, preserving `created_at` and `device_name`.
5. The v3 file replaces the v2 file atomically.

### v1 → v3 (XOR obfuscation)

1. The file is parsed as v1 (`version: 1`, `wrapped_secret_key`, `salt`, `public_key`, `michi_id`).
2. The wrap key is recomputed as `blake3("michi-identity-key-wrap-v1" || hostname || salt)` and the secret key is unwrapped (XOR).
3. The unwrapped key is cross-checked against the stored `public_key` (mismatch → `MigrationFailed`).
4. The stored `michi_id` is cross-checked against `BLAKE3(public_key)` (mismatch → `MigrationFailed`).
5. The keypair is re-encrypted in v3 format, **preserving** `created_at` and `device_name`.
6. The v3 file replaces the v1 file atomically.

### Why v1 and v2 were retired

- **v1** wrapped the secret key with XOR using a key derived from the **hostname**. This was never real encryption — only obfuscation: the wrap key depends only on the hostname and a salt stored in plaintext, XOR with a hash stream is not an authenticated construction, and there is no password binding and no integrity guarantee.
- **v2** replaced XOR with password-derived encryption (ChaCha20-Poly1305 + explicit AAD), but derived the key from a **fast hash** (`blake3(context || salt || password)`), which allows offline GPU brute-force of weak passwords.
- **v3** uses **Argon2id**, a memory-hard KDF, and binds **all** metadata as AAD. This is the current canonical format.

## Corruption handling

Behavior on a corrupted, truncated, tampered, or unsupported file:

- **Explicit error** (`UnsupportedFormat`, `MetadataTampered`, `AuthenticationFailed`, `IdentityMismatch`, or `MigrationFailed`) — never a silent fallback.
- The file is **never overwritten** and the identity is **never regenerated** implicitly. `load_or_generate` only generates when the file is **missing**; any corruption error is propagated.

### Recovery procedure

1. Restore the identity file from a backup.
2. If no backup exists, regeneration is an **explicit operator decision**: delete `identity.msgpack` and call `load_or_generate` again.
3. Be aware that regenerating invalidates previously issued trust (paired devices, stored `michi_id`s).

## Signatures

- Wire signatures are Ed25519 over the **raw payload bytes**, encoded **base64url (86 chars, no padding)**.
- `verify(payload, signature, public_key)` uses `verify_strict` (no malleability tolerance).
- `derive_michi_id(public_key_b64)` derives the identity from an announced key — used by discovery to enforce identity coherence.

## Identity in server/info

`GET /api/v1/server/info` embeds the identity document **all-or-nothing**: `michi_id`, `public_key` and `identity_scheme` are present together or not at all (see `schemas/server-info.schema.json`).

```json
{
  "michi_id": "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
  "public_key": "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ",
  "identity_scheme": "ed25519-blake3-v1",
  "device_name": "Michi Micro Server",
  "created_at": "2026-01-01T00:00:00Z"
}
```

## Reference implementation tests

The crate `crates/michi-identity` covers the identity contract (`src/identity.rs`, `IdentityManager`). The full crate suite is **88 tests**, clippy-clean:

| Test | Verifies |
|------|----------|
| `test_create_and_reload` | generation + reload; `michi_id` is 43 chars |
| `test_persistence_survives_reload` | keypair survives reload; signatures verify |
| `test_wrong_password` | wrong password → `AuthenticationFailed` |
| `test_tampered_ciphertext_fails_without_regen` | tampering fails and identity is NOT regenerated |
| `test_tampered_nonce_fails` | nonce tampering → AEAD failure |
| `test_wrong_aad_fails` | wrong AAD → load fails |
| `test_truncated_file_fails` | truncated file → explicit error |
| `test_michi_id_inconsistent_fails` | stored `michi_id` vs keypair mismatch → `IdentityMismatch` |
| `test_created_at_preserved` | reload preserves creation time |
| `test_load_or_generate_idempotent` | existing file is never regenerated |
| `test_load_or_generate_does_not_heal_corruption` | corruption is not silently healed |
| `test_v1_to_v3_migration` | v1 → v3 migration preserves identity and metadata |
| `test_v2_to_v3_migration` | v2 → v3 migration preserves identity and metadata |
| `test_identity_preserved_after_migration` | migration preserves identity and `created_at` |
| `test_migration_rejects_wrong_legacy_key` | inconsistent legacy file → `MigrationFailed` |
| `test_file_permissions_0600` | file mode is 0600 on Unix |
| `test_sign_and_verify` | sign/verify roundtrip and rejection |
| `test_derive_michi_id_matches` | `derive_michi_id` matches the manager identity |

## Related

- [docs/DISCOVERY.md](DISCOVERY.md) — signed announces and verification pipeline
- [docs/PAIRING.md](PAIRING.md) — TOFU pairing with 6-digit PIN and QR
- [crates/michi-identity/](../crates/michi-identity/) — reference implementation
