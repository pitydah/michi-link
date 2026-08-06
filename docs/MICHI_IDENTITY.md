# Michi Identity

Decentralized identity for the Michi ecosystem. Every device holds an **Ed25519 keypair**; its public identity — the `michi_id` — is derived cryptographically from the public key, so devices recognize each other by identity instead of IPs, volatile UUIDs, or shared secrets.

## Scheme

- **Identity scheme:** `ed25519-blake3-v1`
- **Key type:** Ed25519 (32-byte seed / 32-byte public key), generated from the OS CSPRNG
- **`michi_id` derivation:** `base64url( BLAKE3( public_key_raw_32_bytes ) )` — 32 raw bytes → **43 base64url characters** (URL-safe, no padding)
- **Encodings:** `public_key` and `signature` are base64 (or base64url); `michi_id` is **always base64url**. Hex is NOT a valid canonical representation. SHA-256 is never used for `michi_id`.

```
OS RNG ──→ ed25519-dalek ──→ Keypair
                                │
                                ├── secret_key → ChaCha20-Poly1305 (AEAD)
                                │      key = blake3(context || salt || password)
                                │      AAD  = context || identity_scheme
                                │      └── identity.msgpack (format_version 2)
                                │
                                └── public_key ──→ blake3 ──→ michi_id
                                                         (base64url, 43 chars)
```

## Persistence (format v2)

The identity is persisted as MessagePack in `identity.msgpack` under the application config directory (e.g. `~/.config/michi/identity.msgpack`).

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `format_version` | uint | `2` |
| `identity_scheme` | string | `ed25519-blake3-v1` |
| `michi_id` | string | base64url, 43 chars |
| `public_key` | string | Ed25519 public key, base64 |
| `encrypted_secret` | string | ChaCha20-Poly1305 ciphertext of the 32-byte secret key, base64 |
| `salt` | string | 16 random bytes, base64 |
| `nonce` | string | 12 random bytes, base64 |
| `created_at` | string | RFC 3339 timestamp |
| `device_name` | string | Human-readable device name |

### Encryption

- **Cipher:** ChaCha20-Poly1305 (AEAD).
- **Key derivation:** `key = blake3("michi-identity-file-v2" || salt || password)`.
- **AAD (explicit):** `"michi-identity-file-v2" || "ed25519-blake3-v1"` — binds the ciphertext to the file format and identity scheme, so ciphertext cannot be transplanted into another format.
- **Salt/nonce:** fresh random values per write.

### Atomic writes and permissions

- The file is written via **temp file + `fsync` + rename** in the same directory, so a crash never leaves a truncated target.
- On Unix, permissions are forced to **0600**.
- On load, integrity is cross-checked: the stored `michi_id` must match the identity derived from the decrypted keypair (`KeyMismatch` otherwise).

### Wrong password

A wrong password fails AEAD authentication and returns an explicit `InvalidPassword` error. Nothing is regenerated or overwritten.

## Migration from v1 (automatic)

The legacy v1 format (XOR obfuscation, see below) is detected and migrated **automatically** on load:

1. The file is parsed as v1 (`version: 1`, `wrapped_secret_key`, `salt`, `public_key`, `michi_id`).
2. The wrap key is recomputed as `blake3("michi-identity-key-wrap-v1" || hostname || salt)` and the secret key is unwrapped (XOR).
3. The unwrapped key is cross-checked against the stored `public_key` (mismatch → `MigrationFailed`).
4. The stored `michi_id` is cross-checked against `BLAKE3(public_key)` (mismatch → `MigrationFailed`).
5. The keypair is re-encrypted in v2 format, **preserving** `created_at` and `device_name`.
6. The v2 file replaces the v1 file atomically (same identity — a format upgrade, never a rotation).

### Why v1 was retired

The v1 format wrapped the secret key with XOR using a key derived from the **hostname**:

```
wrapped = secret_key XOR blake3("michi-identity-key-wrap-v1" || hostname || salt)
```

This was never real encryption — it was obfuscation:

- The wrap key depends only on the hostname and a salt stored in plaintext, so anyone with the file and a known hostname can recover the secret key.
- XOR with a hash stream is not an authenticated construction: tampering with the wrapped bytes is undetectable.
- There is no password binding and no integrity guarantee.

v2 replaces it with a password-derived key, authenticated encryption (ChaCha20-Poly1305 + explicit AAD), and atomic writes.

## Corruption handling

Behavior on a corrupted, truncated, tampered, or unsupported file:

- **Explicit error** (`IdentityCorrupted`, `InvalidPassword`, `KeyMismatch`, or `MigrationFailed`) — never a silent fallback.
- The file is **never overwritten** and the identity is **never regenerated** implicitly. `load_or_generate` only generates when the file is **missing**; any corruption error is propagated.

### Recovery procedure

1. Restore the identity file from a backup.
2. If no backup exists, regeneration is an **explicit operator decision**: delete `identity.msgpack` and call `load_or_generate` again.
3. Be aware that regenerating invalidates previously issued trust (paired devices, stored `michi_id`s).

## Signatures

- `sign_standard(payload)` returns `(signature_base64, public_key_base64)`.
- `verify(payload, signature_b64, public_key_b64)` is a static verification accepting base64 and base64url, using `verify_strict` (no malleability tolerance).
- `derive_michi_id(public_key_b64)` derives the identity from an announced key — used by discovery to enforce identity coherence.

## Identity in server/info

`GET /api/v1/server/info` embeds the identity document:

```json
{
  "michi_id": "<base64url 43 chars>",
  "public_key": "<base64>",
  "identity_scheme": "ed25519-blake3-v1",
  "device_name": "Michi Micro Server",
  "created_at": "2026-01-01T00:00:00Z"
}
```

Both `michi_id` and `public_key` are nullable when identity is not initialized.

## Reference implementation tests

The crate `crates/michi-identity` covers the identity contract (`src/identity.rs`):

| Test | Verifies |
|------|----------|
| `test_create_and_reload` | generation + reload; `michi_id` is 43 chars |
| `test_persistence_survives_reload` | keypair survives reload; signatures verify |
| `test_wrong_password` | wrong password → `InvalidPassword` |
| `test_tampered_ciphertext_fails_without_regen` | tampering fails and identity is NOT regenerated |
| `test_tampered_nonce_fails` | nonce tampering → AEAD failure |
| `test_wrong_aad_fails` | wrong AAD → load fails |
| `test_truncated_file_fails` | truncated file → explicit error |
| `test_michi_id_inconsistent_fails` | stored `michi_id` vs keypair mismatch → `KeyMismatch` |
| `test_created_at_preserved` | reload preserves creation time |
| `test_load_or_generate_idempotent` | existing file is never regenerated |
| `test_load_or_generate_does_not_heal_corruption` | corruption is not silently healed |
| `test_migration_from_v1` | v1 → v2 migration preserves identity and metadata |
| `test_migration_rejects_wrong_legacy_key` | inconsistent legacy file → `MigrationFailed` |
| `test_file_permissions_0600` | file mode is 0600 on Unix |
| `test_sign_and_verify` | sign/verify roundtrip and rejection |
| `test_derive_michi_id_matches` | `derive_michi_id` matches the manager identity |

## Related

- [docs/DISCOVERY.md](DISCOVERY.md) — signed announces and verification pipeline
- [docs/PAIRING.md](PAIRING.md) — TOFU pairing with 6-digit PIN and QR
- [crates/michi-identity/](../crates/michi-identity/) — reference implementation
