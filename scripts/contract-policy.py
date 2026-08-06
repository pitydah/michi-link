#!/usr/bin/env python3
"""Contract policy scanner for the Michi Link repository.

Scans the active material of the repo for retired tokens and exits with
status 1 when any are found. Intended to be run in CI as a policy gate.
"""

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Glob patterns scanned under the repo root.
SCAN_GLOBS = (
    "docs/**/*.md",
    "schemas/**/*.json",
    "examples/**/*.json",
    "openapi/**/*.yaml",
)

# Files at the repo root scanned when present.
SCAN_ROOT_FILES = ("README.md", "SECURITY.md")

# Directory names never scanned, even if nested under a scanned root.
SKIP_DIRS = {"node_modules", "target", ".git", ".pytest_cache", "__pycache__"}

# File names never scanned.
SKIP_FILES = {"package-lock.json"}

# A file whose first line contains this marker is treated as historical
# documentation and excluded from the scan entirely.
HISTORICAL_MARKER = "> **HISTORICAL**"

# A line containing this marker is excluded from the scan, so a historical
# document can keep a single offending line without losing its exclusion.
LINE_EXCLUDE_MARKER = "michi-policy:exclude"

# Retired tokens matched as plain substrings (case-sensitive). Applies to
# every scanned file, including prose in docs and descriptions in schemas.
RETIRED_TOKENS = (
    "michi_link_version",
    "michi-big-server",
    "42069",
    "255.255.255.255",
    "pairingId",
    "signedChallenge",
    "pin_proof",
    "pin_proof_signature",
    "pairing_code",
)

# Retired service aliases matched with a trailing boundary lookahead so that
# e.g. "michi-music-player", "michi-micro-server", "michi-stream-standard"
# and "michi-stream-hifi" are NOT reported.
RETIRED_ALIASES = (
    (re.compile(r"michi-player(?![a-z-])"), "michi-player"),
    (re.compile(r"michi-server(?![a-z-])"), "michi-server"),
    (re.compile(r"michi-stream(?![a-z-])"), "michi-stream"),
)

# camelCase wire tokens matched as quoted property occurrences only.
# Bare "deviceId" in prose or in OpenAPI path templates like `{deviceId}`
# (URL surface, not a payload) is never reported.
QUOTED_TOKENS = (
    '"deviceId"',
)

# Retired DTO property keys. Only reported when they appear as JSON/YAML
# object property keys, never as free text (docs legitimately mention file
# paths) and never for the OpenAPI top-level `paths:` key (plural).
PATH_PROPERTY_KEYS = ("path", "file_path", "absolute_path", "local_path", "mount_path")

# Standard base64 characters that must not appear in wire fields: the wire
# format is base64url. Applied to schemas/ and examples/ JSON payloads only;
# openapi/ is intentionally skipped because its YAML patterns already cover
# the wire shape.
WIRE_BASE64_FIELDS = (
    "public_key",
    "michi_id",
    "signature",
    "nonce",
    "challenge_nonce",
    "challenge_signature",
    "server_public_key",
    "server_michi_id",
)

# Matches a JSON property key at the start of a line (pretty-printed JSON).
JSON_PROPERTY_KEY_RE = re.compile(r'^\s*"([^"]+)"\s*:')

# Matches a JSON string-valued property with its value on the same line.
JSON_STRING_FIELD_RE = re.compile(r'^\s*"([^"]+)"\s*:\s*"([^"]*)"')

# Matches an indented YAML property key. The OpenAPI `paths:` key is plural
# and unindented, and path items start with "/", so only schema properties
# can match.
OPENAPI_PATH_KEY_RE = re.compile(r"^\s+(path|file_path|absolute_path|local_path|mount_path):")


def collect_scan_targets():
    """Return the sorted list of files to scan, relative to the repo root."""
    targets = []

    for root_file in SCAN_ROOT_FILES:
        path = REPO_ROOT / root_file
        if path.is_file():
            targets.append(path)

    for pattern in SCAN_GLOBS:
        base_dir, _, tail = pattern.partition("/")
        base = REPO_ROOT / base_dir
        if base.is_dir():
            targets.extend(base.rglob(tail))

    filtered = []
    for path in targets:
        try:
            rel = path.relative_to(REPO_ROOT)
        except ValueError:
            continue
        if not path.is_file():
            continue
        if any(part in SKIP_DIRS for part in rel.parts[:-1]):
            continue
        if path.name in SKIP_FILES:
            continue
        filtered.append(path)

    return sorted(filtered, key=lambda p: p.relative_to(REPO_ROOT).as_posix())


def read_text(path):
    """Decode file bytes as UTF-8 with a latin-1 fallback.

    Returns None for files that look binary (contain null bytes).
    """
    data = path.read_bytes()
    if b"\x00" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return data.decode("latin-1")


def scan_json_structure(lines, text):
    """Return structured findings for a schemas/ or examples/ JSON file.

    Parses the document and walks its objects recursively so that only real
    DTO property keys are reported. Reports:
    - retired DTO property keys (path family) with their JSON path,
    - wire fields carrying standard base64 characters (+ / =).
    Files that do not parse as JSON produce no structured findings.
    """
    try:
        doc = json.loads(text)
    except json.JSONDecodeError:
        return []

    path_keys = {}
    wire_fields = {}

    def walk(node, jsonpath):
        if isinstance(node, dict):
            for key, value in node.items():
                child = f"{jsonpath}.{key}" if jsonpath else f"$.{key}"
                if isinstance(key, str):
                    if key in PATH_PROPERTY_KEYS and key not in path_keys:
                        path_keys[key] = child
                    if (
                        key in WIRE_BASE64_FIELDS
                        and isinstance(value, str)
                        and any(c in value for c in "+/=")
                    ):
                        wire_fields[key] = child
                walk(value, child)
        elif isinstance(node, list):
            for index, item in enumerate(node):
                walk(item, f"{jsonpath}[{index}]")

    walk(doc, "")

    findings = []
    seen = set()

    for lineno, line in enumerate(lines, start=1):
        if LINE_EXCLUDE_MARKER in line:
            continue
        key_match = JSON_PROPERTY_KEY_RE.match(line)
        if key_match and key_match.group(1) in path_keys:
            entry = (lineno, path_keys[key_match.group(1)])
            if entry not in seen:
                seen.add(entry)
                findings.append({"line": lineno, "token": entry[1]})
            continue
        field_match = JSON_STRING_FIELD_RE.match(line)
        if field_match and field_match.group(1) in wire_fields:
            entry = (
                lineno,
                f"non-canonical base64 in wire field {field_match.group(1)}",
            )
            if entry not in seen:
                seen.add(entry)
                findings.append({"line": lineno, "token": entry[1]})

    return findings


def scan_file(path):
    """Return (findings, status) for a single file.

    Status is "scanned", "binary" or "historical". Findings are
    {"line": int, "token": str} entries, deduplicated per line and token.
    """
    text = read_text(path)
    if text is None:
        return [], "binary"

    lines = text.splitlines()
    if lines and HISTORICAL_MARKER in lines[0]:
        return [], "historical"

    findings = []
    seen = set()

    def add(line, token):
        entry = (line, token)
        if entry not in seen:
            seen.add(entry)
            findings.append({"line": line, "token": token})

    for lineno, line in enumerate(lines, start=1):
        if LINE_EXCLUDE_MARKER in line:
            continue
        for token in RETIRED_TOKENS:
            if token in line:
                add(lineno, token)
        for regex, alias in RETIRED_ALIASES:
            if regex.search(line):
                add(lineno, alias)
        for token in QUOTED_TOKENS:
            if token in line:
                add(lineno, token)

    rel = path.relative_to(REPO_ROOT).as_posix()

    if path.suffix == ".json" and rel.startswith(("schemas/", "examples/")):
        for finding in scan_json_structure(lines, text):
            add(finding["line"], finding["token"])

    if path.suffix == ".yaml" and rel.startswith("openapi/"):
        for lineno, line in enumerate(lines, start=1):
            if LINE_EXCLUDE_MARKER in line:
                continue
            key_match = OPENAPI_PATH_KEY_RE.match(line)
            if key_match:
                add(lineno, key_match.group(1))

    return sorted(findings, key=lambda f: (f["line"], f["token"])), "scanned"


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Scan the repo's active material for retired tokens."
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit a structured JSON report instead of human-readable output.",
    )
    args = parser.parse_args(argv)

    targets = collect_scan_targets()

    findings = []
    scanned = 0
    excluded = 0
    binary_skipped = 0

    for path in targets:
        rel = path.relative_to(REPO_ROOT).as_posix()
        file_findings, status = scan_file(path)
        if status == "scanned":
            scanned += 1
            for finding in file_findings:
                finding["file"] = rel
                findings.append(finding)
        elif status == "historical":
            excluded += 1
        elif status == "binary":
            binary_skipped += 1

    findings.sort(key=lambda f: (f["file"], f["line"], f["token"]))
    failed_files = {f["file"] for f in findings}
    pass_scan = not findings

    if args.json:
        report = {
            "pass": pass_scan,
            "scanned_files": scanned,
            "excluded_files": excluded,
            "binary_skipped_files": binary_skipped,
            "findings": findings,
        }
        print(json.dumps(report, indent=2))
    else:
        for finding in findings:
            print(f"POLICY FAIL {finding['file']}:{finding['line']}: {finding['token']}")
        if pass_scan:
            print(f"Contract policy: PASS ({scanned} files scanned)")
        else:
            print(
                f"Contract policy: FAIL ({len(findings)} findings in {len(failed_files)} files)"
            )

    return 0 if pass_scan else 1


if __name__ == "__main__":
    sys.exit(main())
