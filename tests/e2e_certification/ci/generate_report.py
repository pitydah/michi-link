#!/usr/bin/env python3
"""Deterministic certification report generator (P1-03).

Writes the certification report in the EXACT mission shape, with no
timestamps or other variable fields, so two runs at the same commits produce
byte-identical output:

    {
      "certification": "michi-link-alpha1",
      "gate": {"MOCK_PASS": true},
      "commits": {
        "michi_link": {"commit": "<sha-real>"},
        "michi_music_stream": {"commit": "<sha-real>"}
      },
      "environment": "canonical receiver simulator",
      "device_e2e": false
    }

The report certifies the canonical receiver SIMULATOR only: it never asserts
DEVICE_E2E_PASS / NETWORK_E2E_PASS / hardware certification / production
readiness (`device_e2e` is always false).
"""

import argparse
import json
import os
import re
import sys

SHA_RE = re.compile(r"^[0-9a-f]{40}$")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--link-commit", required=True,
                        help="exact Michi Link commit SHA under test")
    parser.add_argument("--stream-commit", required=True,
                        help="exact Michi Music Stream commit SHA under test")
    parser.add_argument("--out", required=True,
                        help="destination path (created atomically)")
    args = parser.parse_args()

    for label, sha in (("link", args.link_commit), ("stream", args.stream_commit)):
        if not SHA_RE.match(sha):
            print(f"ERROR: {label} commit is not a full 40-hex SHA: {sha!r}",
                  file=sys.stderr)
            return 1

    report = {
        "certification": "michi-link-alpha1",
        "gate": {"MOCK_PASS": True},
        "commits": {
            "michi_link": {"commit": args.link_commit},
            "michi_music_stream": {"commit": args.stream_commit},
        },
        "environment": "canonical receiver simulator",
        "device_e2e": False,
    }
    payload = json.dumps(report, indent=2) + "\n"

    out_dir = os.path.dirname(os.path.abspath(args.out))
    os.makedirs(out_dir, exist_ok=True)
    tmp_path = f"{args.out}.tmp"
    with open(tmp_path, "w", encoding="utf-8") as f:
        f.write(payload)
    os.replace(tmp_path, args.out)
    print(f"Certification report written: {args.out}")
    print(payload, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
