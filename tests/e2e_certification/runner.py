#!/usr/bin/env python3
"""
Michi Link E2E Certification Runner.

Usage:
    python runner.py scenarios/mobile_player.yml --server-host 192.168.1.100 --server-port 8400
    python runner.py scenarios/mobile_micro.yml --server-host 192.168.1.101 --server-port 8500
    python runner.py scenarios/micro_autonomous.yml --server-host 192.168.1.101 --server-port 8500
    python runner.py scenarios/player_micro_import.yml --player-host 192.168.1.100 --micro-host 192.168.1.101
"""

import argparse
import json
import os
import sys
import yaml
import requests
from datetime import datetime, timezone
from assertions import check
from report import ReportBuilder

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
REPORTS_DIR = os.path.join(BASE_DIR, "reports")
CERTIFICATION_LEVELS = {"NOT_TESTED", "UNIT_PASS", "MOCK_PASS", "MANUAL_PASS", "E2E_PASS", "FAIL"}


def make_requests_session():
    session = requests.Session()
    session.headers.update({"Content-Type": "application/json"})
    session.timeout = 10
    return session


def resolve_server(server_ref, args):
    """Resolve a server reference from scenario YAML to a (host, port) tuple."""
    if server_ref == "player":
        return args.server_host or "127.0.0.1", args.server_port or 8400
    elif server_ref == "micro":
        return args.server_host or "127.0.0.1", args.server_port or 8500
    elif server_ref == "player_actual":
        return args.player_host or "127.0.0.1", args.player_port or 8400
    elif server_ref == "micro_actual":
        return args.micro_host or "127.0.0.1", args.micro_port or 8500
    else:
        raise ValueError(f"Unknown server reference: {server_ref}")


def run_check(session, check_def, servers, args):
    """Execute a single check from a scenario definition."""
    method = check_def.get("method", "GET").upper()
    path = check_def["path"]
    server_ref = check_def.get("server")
    host, port = resolve_server(server_ref, args)
    url = f"http://{host}:{port}{path}"
    headers = {}
    body = check_def.get("body")
    token = check_def.get("token")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    expected_status = check_def.get("expected_status", 200)
    expected_json = check_def.get("expected_json")
    not_expected_fields = check_def.get("not_expected_fields")
    extract_vars = check_def.get("extract", {})
    extra_headers = check_def.get("headers", {})
    headers.update(extra_headers)

    try:
        if method == "GET":
            resp = session.get(url, headers=headers)
        elif method == "POST":
            resp = session.post(url, headers=headers, json=body)
        elif method == "PUT":
            resp = session.put(url, headers=headers, json=body)
        elif method == "DELETE":
            resp = session.delete(url, headers=headers)
        else:
            return {
                "name": check_def["name"],
                "status": "fail",
                "detail": f"Unsupported method: {method}",
            }

        result = check(
            name=check_def["name"],
            status_code=resp.status_code,
            expected_status=expected_status,
            body=resp,
            expected_json=expected_json,
            not_expected_fields=not_expected_fields,
        )

        # Extract variables from response for chaining
        if extract_vars and resp.ok:
            try:
                data = resp.json()
                for var_name, json_path in extract_vars.items():
                    value = data
                    for key in json_path.split("."):
                        if isinstance(value, dict):
                            value = value.get(key)
                        else:
                            value = None
                            break
                    if value is not None:
                        args.__dict__[f"var_{var_name}"] = str(value)
            except Exception:
                pass

        return result

    except requests.exceptions.ConnectionError as e:
        return {
            "name": check_def["name"],
            "status": "fail",
            "detail": f"Connection refused: {e}",
        }
    except Exception as e:
        return {
            "name": check_def["name"],
            "status": "fail",
            "detail": str(e),
        }


def run_scenario(scenario_path, args):
    """Run a full scenario from a YAML file."""
    with open(scenario_path, "r") as f:
        scenario = yaml.safe_load(f)

    session = make_requests_session()
    report = ReportBuilder(
        scenario_id=scenario["id"],
        scenario_name=scenario["name"],
        server_type=scenario.get("server_type", "unknown"),
        client_type=scenario.get("client_type", "runner"),
        certifier=args.certified_by or os.environ.get("USER", "unknown"),
    )

    for check_def in scenario["checks"]:
        result = run_check(session, check_def, scenario.get("servers", {}), args)
        report.add_check(result["name"], result["status"], result["detail"])
        if result["status"] == "fail":
            print(f"  FAIL: {result['name']} — {result['detail']}")
            if args.fail_fast:
                break
        else:
            print(f"  PASS: {result['name']} — {result['detail']}")

    report_path = report.save(REPORTS_DIR)
    print(f"\nReport saved: {report_path}")
    print(f"Result: {report.status}")
    return report.status == "pass"


def main():
    parser = argparse.ArgumentParser(description="Michi Link E2E Certification Runner")
    parser.add_argument("scenario", help="Path to scenario YAML file")
    parser.add_argument("--server-host", help="Server hostname/IP")
    parser.add_argument("--server-port", type=int, help="Server port")
    parser.add_argument("--player-host", help="Player hostname/IP")
    parser.add_argument("--player-port", type=int, default=8400, help="Player port (default: 8400)")
    parser.add_argument("--micro-host", help="Micro Server hostname/IP")
    parser.add_argument("--micro-port", type=int, default=8500, help="Micro Server port (default: 8500)")
    parser.add_argument("--certified-by", default=None, help="Certifier email or username")
    parser.add_argument("--fail-fast", action="store_true", help="Stop on first failure")
    parser.add_argument("--token", default=None, help="Bearer token for authenticated requests")

    args = parser.parse_args()

    if not os.path.exists(args.scenario):
        print(f"Scenario file not found: {args.scenario}")
        sys.exit(1)

    success = run_scenario(args.scenario, args)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
