#!/usr/bin/env python3
"""Bundle drift verification (P1-03).

Compares the normative Link bundle (contracts/receiver-v1-lite) with the
vendored Stream copy (contracts/michi-link in the michi-music-stream
checkout). Both bundles must be byte-identical and their manifest-internal
sha256 hashes must verify; any drift fails the job (exit code 1).
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path


def tree_hash(root):
    entries = []
    for path in sorted(Path(root).rglob("*")):
        if path.is_file():
            rel = path.relative_to(root).as_posix()
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            entries.append(f"{rel}\t{digest}")
    return entries


def verify_manifest_hashes(root):
    manifest_path = Path(root) / "manifest.json"
    if not manifest_path.is_file():
        return ["manifest.json missing"]
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"manifest.json unreadable: {exc}"]
    problems = []
    for entry in manifest.get("files", []):
        path = Path(root) / entry["path"]
        if not path.is_file():
            problems.append(f"{entry['path']}: listed in the manifest but missing")
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != entry["sha256"]:
            problems.append(f"{entry['path']}: sha256 does not match the manifest")
    return problems


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("link_bundle", help="Link normative bundle directory")
    parser.add_argument("stream_bundle", help="Stream vendored bundle directory")
    args = parser.parse_args()

    link = Path(args.link_bundle)
    stream = Path(args.stream_bundle)
    if not (link / "manifest.json").is_file():
        print(f"ERROR: Link bundle not found at {link}", file=sys.stderr)
        return 1
    if not (stream / "manifest.json").is_file():
        print(f"ERROR: Stream vendored bundle not found at {stream}", file=sys.stderr)
        return 1

    problems = verify_manifest_hashes(link) + verify_manifest_hashes(stream)
    link_tree = tree_hash(link)
    stream_tree = tree_hash(stream)

    version_link = (link / "VERSION").read_text(encoding="utf-8").strip()
    version_stream = (stream / "VERSION").read_text(encoding="utf-8").strip()
    print(f"Link bundle version: {version_link}")
    print(f"Stream vendored bundle version: {version_stream}")
    print(f"Link bundle tree sha256: "
          f"{hashlib.sha256(''.join(link_tree).encode()).hexdigest()}")
    print(f"Stream vendored bundle tree sha256: "
          f"{hashlib.sha256(''.join(stream_tree).encode()).hexdigest()}")

    if link_tree != stream_tree:
        only_link = set(link_tree) - set(stream_tree)
        only_stream = set(stream_tree) - set(link_tree)
        problems.append(
            f"bundle drift: {len(only_link)} file(s) differ in Link, "
            f"{len(only_stream)} file(s) differ in Stream"
        )
        for item in sorted(only_link)[:10]:
            print(f"  link-only: {item}", file=sys.stderr)
        for item in sorted(only_stream)[:10]:
            print(f"  stream-only: {item}", file=sys.stderr)

    if problems:
        for problem in problems:
            print(f"ERROR: {problem}", file=sys.stderr)
        return 1
    print("Bundle verification: the Link normative bundle and the Stream "
          "vendored copy are byte-identical (manifest sha256 hashes verified).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
