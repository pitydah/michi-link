#!/usr/bin/env python3
"""Legacy receiver route scan (P1-03, gate 11).

Scans the certification suite and the CI workflow for references to the
RETIRED legacy receiver dialect. The only allowed occurrences are the
canonical negative checks of the E2E-09 scenario (checks whose name starts
with `legacy_` and that expect the canonical 404 NOT_FOUND). Any other
occurrence — in scenarios, runner code, docs, or the CI workflow — fails the
job (exit code 1).

This tool lives under ci/ and is excluded from its own scan (the patterns
are its subject matter, not references to the legacy dialect).
"""

import argparse
import sys
from pathlib import Path

import yaml

LEGACY_PATTERNS = (
    "/api/v1/receiver/",
    "/api/v1/receiver-lite/volume",
    "/api/v1/receiver-lite/info",
    "/api/v1/receiver-lite/config",
)

SCENARIO_FILE = "tests/e2e_certification/scenarios/micro_stream_receiver.yml"
WORKFLOW_FILE = ".github/workflows/stream-interop.yml"

SKIP_DIRS = {"reports", "ci-artifacts", "__pycache__", "ci"}


def find_occurrences(text):
    return [pattern for pattern in LEGACY_PATTERNS if pattern in text]


def check_scenario(scenario_path):
    problems = []
    try:
        scenario = yaml.safe_load(Path(scenario_path).read_text(encoding="utf-8"))
    except Exception as exc:
        return [f"cannot parse {scenario_path}: {exc}"]
    for check in scenario.get("checks", []):
        raw = str(check)
        if not find_occurrences(raw):
            continue
        name = check.get("name", "")
        expected = check.get("expected_status")
        if not name.startswith("legacy_") or expected != 404:
            problems.append(
                f"{scenario_path}: legacy route referenced by check '{name}' "
                f"(expected_status={expected}); only legacy_* checks expecting "
                f"the canonical 404 NOT_FOUND may reference legacy routes"
            )
    return problems


def check_plain_file(path):
    try:
        text = Path(path).read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        return [f"cannot read {path}: {exc}"]
    matches = find_occurrences(text)
    if matches:
        return [f"{path}: legacy route reference(s): {', '.join(matches)}"]
    return []


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "link_root",
        nargs="?",
        default=".",
        help="Michi Link checkout root (default: current directory)",
    )
    args = parser.parse_args()

    root = Path(args.link_root).resolve()
    problems = []

    scenario = root / SCENARIO_FILE
    if scenario.is_file():
        problems.extend(check_scenario(scenario))
    else:
        problems.append(f"scenario not found: {scenario}")

    workflow = root / WORKFLOW_FILE
    if workflow.is_file():
        problems.extend(check_plain_file(workflow))
    else:
        problems.append(f"CI workflow not found: {workflow}")

    suite_dir = root / "tests" / "e2e_certification"
    for path in sorted(suite_dir.rglob("*")):
        if not path.is_file() or path.suffix == ".pyc":
            continue
        if path == scenario:
            continue
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        problems.extend(check_plain_file(path))

    if problems:
        for problem in problems:
            print(f"ERROR: {problem}", file=sys.stderr)
        return 1
    print("Legacy route scan: no legacy route references outside the "
          "canonical negative checks of E2E-09.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
