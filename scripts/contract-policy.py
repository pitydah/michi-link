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

# Retired tokens matched as plain substrings (case-sensitive).
RETIRED_TOKENS = (
    "michi_link_version",
    "michi-big-server",
    "42069",
    "255.255.255.255",
)

# Retired service aliases matched with a trailing boundary lookahead so that
# e.g. "michi-music-player", "michi-micro-server", "michi-stream-standard"
# and "michi-stream-hifi" are NOT reported.
RETIRED_ALIASES = (
    (re.compile(r"michi-player(?![a-z-])"), "michi-player"),
    (re.compile(r"michi-server(?![a-z-])"), "michi-server"),
    (re.compile(r"michi-stream(?![a-z-])"), "michi-stream"),
)


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
    for lineno, line in enumerate(lines, start=1):
        if LINE_EXCLUDE_MARKER in line:
            continue
        seen = set()
        for token in RETIRED_TOKENS:
            if token in line:
                seen.add(token)
        for regex, alias in RETIRED_ALIASES:
            if regex.search(line):
                seen.add(alias)
        for token in sorted(seen):
            findings.append({"line": lineno, "token": token})

    return findings, "scanned"


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
