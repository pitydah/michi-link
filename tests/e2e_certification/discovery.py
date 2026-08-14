"""Cryptographic identity + signed discovery announce helpers for the E2E runner.

Implements the canonical michi-identity derivation and the signed discovery
announce verification pipeline (contracts/receiver-v1-lite,
discovery-announce.schema.json):

  - michi_id = base64url-nopad(BLAKE3(public_key))
  - Ed25519 signature over the canonical announce bytes (byte-identical to
    the firmware builder and the Rust DiscoveryEngine::canonical_bytes)
  - timestamp freshness window (+-90 s)
  - (michi_id, nonce) replay rejection
"""

import base64
import secrets
import time
import uuid

import blake3
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)

_B64URL_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"

FRESHNESS_WINDOW_MS = 90_000


def b64url_nopad(raw_bytes):
    return base64.urlsafe_b64encode(raw_bytes).decode("ascii").rstrip("=")


def b64url_decode(value):
    if not isinstance(value, str) or not value or "=" in value:
        raise ValueError("strict base64url without padding required")
    if any(ch not in _B64URL_ALPHABET for ch in value):
        raise ValueError("invalid base64url character")
    return base64.urlsafe_b64decode(value + "=" * (-len(value) % 4))


def derive_michi_id(public_key_bytes):
    return b64url_nopad(blake3.blake3(public_key_bytes).digest())


def canonical_bytes(announce):
    """Canonical signed-announce bytes (lexicographic key order, signature
    excluded). Byte-identical to the firmware builder and the Rust
    DiscoveryEngine::canonical_bytes. Requires exactly one role and the
    heartbeat/session/volume feature set of the canonical vector."""
    features = announce["features"]
    return (
        '{"api_version":"%s","device_id":"%s",'
        '"features":{"heartbeat":%s,"session":%s,"volume":%s},'
        '"host":"%s","michi_id":"%s","name":"%s","nonce":"%s",'
        '"port":%u,"public_key":"%s","roles":["%s"],"service":"%s",'
        '"timestamp_ms":%d}'
        % (
            announce["api_version"],
            announce["device_id"],
            "true" if features["heartbeat"] else "false",
            "true" if features["session"] else "false",
            "true" if features["volume"] else "false",
            announce["host"],
            announce["michi_id"],
            announce["name"],
            announce["nonce"],
            announce["port"],
            announce["public_key"],
            announce["roles"][0],
            announce["service"],
            announce["timestamp_ms"],
        )
    ).encode("utf-8")


def ed25519_verify(public_key_b64, signature_b64, message_bytes):
    public_key = Ed25519PublicKey.from_public_bytes(b64url_decode(public_key_b64))
    try:
        public_key.verify(b64url_decode(signature_b64), message_bytes)
        return True
    except InvalidSignature:
        return False


def new_identity():
    """Fresh Ed25519 identity: (private_key, raw public key bytes)."""
    private_key = Ed25519PrivateKey.generate()
    public_bytes = private_key.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    return private_key, public_bytes


def build_announce(host, port, device_id=None, now_ms=None):
    """Builds a freshly signed canonical discovery announce (dynamic identity,
    dynamic nonce, dynamic timestamp). Returns (announce, private_key)."""
    private_key, public_bytes = new_identity()
    michi_id = derive_michi_id(public_bytes)
    public_key = b64url_nopad(public_bytes)
    announce = {
        "device_id": device_id or str(uuid.uuid4()),
        "name": "Michi Stream E2E",
        "service": "michi-stream-standard",
        "roles": ["audio_receiver"],
        "api_version": "v1-lite",
        "host": host,
        "port": port,
        "features": {"heartbeat": True, "session": True, "volume": True},
        "michi_id": michi_id,
        "public_key": public_key,
        "nonce": b64url_nopad(secrets.token_bytes(16)),
        "timestamp_ms": now_ms if now_ms is not None else int(time.time() * 1000),
    }
    announce["signature"] = b64url_nopad(private_key.sign(canonical_bytes(announce)))
    return announce, private_key


def corrupt_b64url_char(value):
    """Returns the same base64url string with one character replaced by the
    next valid alphabet character (always different, still valid). The
    changed character is in the middle of the string so it contributes
    decoded bits (the trailing characters of an 86-char signature only
    carry discarded padding bits)."""
    chars = _B64URL_ALPHABET
    index = len(value) // 2
    replacement = chars[(chars.index(value[index]) + 1) % len(chars)]
    return value[:index] + replacement + value[index + 1:]


class DiscoveryVerifier:
    """Controller-side discovery verification pipeline (contract section 2.2).

    Rejects: stale timestamps (+-90 s), replayed (michi_id, nonce) pairs,
    identity incoherence and invalid Ed25519 signatures."""

    def __init__(self, now_ms=None):
        self._now_ms = now_ms or (lambda: int(time.time() * 1000))
        self._seen = set()

    def accept(self, announce):
        """Returns (ok, reason)."""
        timestamp = announce.get("timestamp_ms")
        if not isinstance(timestamp, int):
            return False, "timestamp_ms missing"
        delta = abs(self._now_ms() - timestamp)
        if delta > FRESHNESS_WINDOW_MS:
            return False, f"timestamp outside the +-90 s window (delta {delta} ms)"
        key = (announce.get("michi_id"), announce.get("nonce", ""))
        if key in self._seen:
            return False, "replay rejected: (michi_id, nonce) already accepted"
        try:
            public_key_bytes = b64url_decode(announce.get("public_key", ""))
        except ValueError as exc:
            return False, f"public_key invalid: {exc}"
        if derive_michi_id(public_key_bytes) != announce.get("michi_id"):
            return False, "michi_id does not correspond to public_key"
        try:
            signature_ok = ed25519_verify(
                announce.get("public_key", ""),
                announce.get("signature", ""),
                canonical_bytes(announce),
            )
        except (ValueError, KeyError):
            signature_ok = False
        if not signature_ok:
            return False, "Ed25519 signature does not verify over the canonical bytes"
        self._seen.add(key)
        return True, "accepted"
