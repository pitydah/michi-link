#!/usr/bin/env python3
"""Build the immutable receiver v1-lite conformance bundle.

Deterministic by construction: reads canonical sources, sorts every file
list and emits the bundle plus a sorted SHA-256 manifest with no timestamps,
so two builds from the same sources are byte-for-byte identical.

Bundle layout:

    contracts/receiver-v1-lite/
      VERSION
      UPSTREAM_COMMIT
      manifest.json
      openapi/michi-link-v1.yaml
      schemas/*.schema.json
      examples/positive/*.json
      examples/negative/*.json
      vectors/identity/*.json
      vectors/discovery/*.json
      vectors/pairing/*.json

Sources:
- `openapi/michi-link-v1.yaml` and `schemas/*.schema.json` are copied
  byte-for-byte from the canonical contract surface.
- Vector and example payloads come from `tests/vectors/receiver-v1-lite/`,
  generated deterministically by the `generate_contract_vectors` example.
- `UPSTREAM_COMMIT` records the latest commit that touched the canonical
  contract surface (openapi/ + schemas/); it is stable across rebuilds of
  the same bundle.
- `manifest.json` lists every bundled file (except itself) with its SHA-256,
  sorted by path.
"""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BUNDLE_DIR = REPO_ROOT / "contracts" / "receiver-v1-lite"
SOURCE_VECTORS = REPO_ROOT / "tests" / "vectors" / "receiver-v1-lite"
BUNDLE_VERSION = "1.0.0-alpha.1"

# Source subdirectory (under tests/vectors/receiver-v1-lite) -> bundle
# subdirectory. Files are copied in sorted order.
VECTOR_SUBDIRS = (
    ("identity", "vectors/identity"),
    ("discovery", "vectors/discovery"),
    ("pairing", "vectors/pairing"),
    ("examples/positive", "examples/positive"),
    ("examples/negative", "examples/negative"),
)


def sha256(path: Path) -> str:
    """Return the lowercase hex SHA-256 digest of a file."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 16), b""):
            digest.update(chunk)
    return digest.hexdigest()


def upstream_commit() -> str:
    """Latest commit that touched the canonical contract surface."""
    output = subprocess.run(
        ["git", "log", "-1", "--format=%H", "--", "openapi/michi-link-v1.yaml", "schemas"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    commit = output.strip()
    if not commit:
        sys.exit("error: could not resolve the upstream contract commit")
    return commit


def source_entries():
    """Return (source_path, bundle_rel_path) pairs, sorted by bundle path."""
    entries = [
        (REPO_ROOT / "openapi" / "michi-link-v1.yaml", "openapi/michi-link-v1.yaml"),
    ]
    for schema in sorted((REPO_ROOT / "schemas").glob("*.schema.json")):
        entries.append((schema, f"schemas/{schema.name}"))
    for source_subdir, bundle_subdir in VECTOR_SUBDIRS:
        source_dir = SOURCE_VECTORS / source_subdir
        for vector in sorted(source_dir.glob("*.json")):
            entries.append((vector, f"{bundle_subdir}/{vector.name}"))
    entries.sort(key=lambda entry: entry[1])
    return entries


def build() -> None:
    """Rebuild the bundle deterministically and print a short summary."""
    files = {
        "VERSION": (BUNDLE_VERSION + "\n").encode("utf-8"),
        "UPSTREAM_COMMIT": (upstream_commit() + "\n").encode("utf-8"),
    }
    for source, rel in source_entries():
        if not source.is_file():
            sys.exit(f"error: missing source file {source}")
        files[rel] = source.read_bytes()

    # Remove any bundled file that is no longer part of the exact set, so the
    # bundle directory always matches the manifest (idempotent regeneration).
    expected = set(files) | {"manifest.json"}
    for existing in sorted(BUNDLE_DIR.rglob("*")):
        if existing.is_file():
            rel = existing.relative_to(BUNDLE_DIR).as_posix()
            if rel not in expected:
                existing.unlink()

    manifest_files = []
    for rel in sorted(files):
        target = BUNDLE_DIR / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(files[rel])
        manifest_files.append({"path": rel, "sha256": sha256(target)})

    manifest = {
        "version": BUNDLE_VERSION,
        "upstream_commit": files["UPSTREAM_COMMIT"].decode("utf-8").strip(),
        "files": manifest_files,
    }
    manifest_path = BUNDLE_DIR / "manifest.json"
    with manifest_path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(manifest, indent=2) + "\n")

    print(
        f"receiver v1-lite bundle {BUNDLE_VERSION}: "
        f"{len(manifest_files)} files + manifest written to {BUNDLE_DIR}"
    )


if __name__ == "__main__":
    build()
