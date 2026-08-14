#!/usr/bin/env python3
"""
Michi Link E2E Certification Runner.

Usage:
    python runner.py scenarios/mobile_player.yml --server-host 192.168.1.100 --server-port 8400
    python runner.py scenarios/mobile_micro.yml --server-host 192.168.1.101 --server-port 8500
    python runner.py scenarios/micro_autonomous.yml --server-host 192.168.1.101 --server-port 8500
    python runner.py scenarios/player_micro_import.yml --player-host 192.168.1.100 --micro-host 192.168.1.101

    # Canonical stream receiver scenario (E2E-09): embedded simulator harness (default)
    python runner.py scenarios/micro_stream_receiver.yml

    # ... or against a manually started simulator process (harness contract: HARNESS.md)
    python runner.py scenarios/micro_stream_receiver.yml \\
        --stream-host 127.0.0.1 --stream-port 8600 --stream-pin-log /tmp/stream-sim.log
"""

import argparse
import json
import os
import re
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.parse
from pathlib import Path

import requests
import yaml

from assertions import check
from contract_schemas import ContractSchemas
from discovery import (
    DiscoveryVerifier,
    FRESHNESS_WINDOW_MS,
    b64url_decode,
    b64url_nopad,
    build_announce,
    canonical_bytes,
    corrupt_b64url_char,
    derive_michi_id,
    ed25519_verify,
    new_identity,
)
from report import ReportBuilder
from rtp import (
    RTP_PAYLOAD_BYTES,
    RTP_PAYLOAD_TYPE,
    build_packet,
    parse_packet,
    pcm10ms_payload,
)
from stream_harness import (
    EmbeddedStreamHarness,
    ExternalStreamHarness,
    default_sim_dir,
)

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
REPORTS_DIR = os.path.join(BASE_DIR, "reports")
BUNDLE_DIR = Path(BASE_DIR).parent.parent / "contracts" / "receiver-v1-lite"
VECTORS_DIR = BUNDLE_DIR / "vectors" / "discovery"
CERTIFICATION_LEVELS = {"NOT_TESTED", "UNIT_PASS", "MOCK_PASS", "MANUAL_PASS", "E2E_PASS", "FAIL"}

_FULL_TEMPLATE_RE = re.compile(r"^\s*\{\{\s*([A-Za-z0-9_]+)\s*\}\}\s*$")
_TEMPLATE_RE = re.compile(r"\{\{\s*[A-Za-z0-9_]+\s*\}\}")


def _dotted_get(data, path):
    value = data
    for key in path.split("."):
        if isinstance(value, dict):
            value = value.get(key)
        else:
            return None
    return value


def render(value, variables):
    """Recursive template rendering: `{{ var }}` placeholders are resolved
    against the scenario variables. A whole-string placeholder keeps the raw
    variable type; embedded placeholders are stringified."""
    if isinstance(value, str):
        full = _FULL_TEMPLATE_RE.match(value)
        if full:
            name = full.group(1)
            if name not in variables:
                raise ValueError(f"undefined variable: {name}")
            return variables[name]

        def substitute(match):
            name = match.group(0)[2:-2].strip()
            if name not in variables:
                raise ValueError(f"undefined variable: {name}")
            return str(variables[name])

        return _TEMPLATE_RE.sub(substitute, value)
    if isinstance(value, dict):
        return {key: render(item, variables) for key, item in value.items()}
    if isinstance(value, list):
        return [render(item, variables) for item in value]
    return value


def _pass(name, detail):
    return {"name": name, "status": "pass", "detail": detail}


def _fail(name, detail):
    return {"name": name, "status": "fail", "detail": detail}


class ScenarioRuntime:
    def __init__(self, args, harness):
        self.args = args
        self.harness = harness
        self.variables = {}
        self.schemas = ContractSchemas()


def make_requests_session():
    session = requests.Session()
    session.headers.update({"Content-Type": "application/json"})
    session.timeout = 10
    return session


def resolve_server(server_ref, args, runtime=None):
    """Resolve a server reference from scenario YAML to a (host, port) tuple."""
    if server_ref == "player":
        return args.server_host or "127.0.0.1", args.server_port or 8400
    elif server_ref == "micro":
        return args.server_host or "127.0.0.1", args.server_port or 8500
    elif server_ref == "player_actual":
        return args.player_host or "127.0.0.1", args.player_port or 8400
    elif server_ref == "micro_actual":
        return args.micro_host or "127.0.0.1", args.micro_port or 8500
    elif server_ref == "stream":
        if runtime is not None and runtime.harness is not None:
            return runtime.harness.base_host, runtime.harness.base_port
        return args.stream_host or "127.0.0.1", args.stream_port or 8080
    else:
        raise ValueError(f"Unknown server reference: {server_ref}")


def run_check(runtime, session, check_def):
    """Execute a single HTTP check from a scenario definition."""
    variables = runtime.variables
    method = check_def.get("method", "GET").upper()
    try:
        path = render(check_def["path"], variables)
        query_def = check_def.get("query") or {}
        query = {key: render(value, variables) for key, value in query_def.items()}
        server_ref = check_def.get("server")
        host, port = resolve_server(server_ref, runtime.args, runtime)
        url = f"http://{host}:{port}{path}"
        if query:
            url += "?" + urllib.parse.urlencode(query)
        headers = {}
        for key, value in (check_def.get("headers") or {}).items():
            headers[key] = render(value, variables)
        body = render(check_def.get("body"), variables)
        token = check_def.get("token") or runtime.args.token
        if token:
            headers["Authorization"] = f"Bearer {render(token, variables)}"
        expected_status = check_def.get("expected_status", 200)
        expected_json = dict(check_def.get("expected_json") or {})
        for key, value in (check_def.get("assert_eq") or {}).items():
            expected_json[key] = render(value, variables)
        not_expected_fields = check_def.get("not_expected_fields")
        extract_vars = check_def.get("extract") or {}
        no_body = check_def.get("no_body", False)
        assert_ranges = check_def.get("assert_ranges")

        if method == "GET":
            resp = session.get(url, headers=headers)
        elif method == "POST":
            resp = session.post(url, headers=headers, json=body)
        elif method == "PUT":
            resp = session.put(url, headers=headers, json=body)
        elif method == "PATCH":
            resp = session.patch(url, headers=headers, json=body)
        elif method == "DELETE":
            resp = session.delete(url, headers=headers)
        else:
            return _fail(check_def["name"], f"Unsupported method: {method}")

        result = check(
            name=check_def["name"],
            status_code=resp.status_code,
            expected_status=expected_status,
            body=resp,
            expected_json=expected_json,
            not_expected_fields=not_expected_fields,
            no_body=no_body,
            assert_ranges=assert_ranges,
        )

        if result["status"] == "pass":
            schema = check_def.get("validate_schema")
            if schema:
                try:
                    data = resp.json()
                    errors = runtime.schemas.validate(schema, data)
                except Exception as exc:
                    errors = [f"non-JSON response: {exc}"]
                if errors:
                    result["status"] = "fail"
                    result["detail"] = f"schema {schema}: {errors[0]}"

        if result["status"] == "pass":
            spec = check_def.get("verify_michi_id")
            if spec:
                try:
                    data = resp.json()
                    michi_id = _dotted_get(data, spec["michi_id"])
                    public_key = _dotted_get(data, spec["public_key"])
                    derived = derive_michi_id(b64url_decode(public_key))
                    if derived != michi_id:
                        result["status"] = "fail"
                        result["detail"] = (
                            f"michi_id does not correspond to public_key: "
                            f"derived {derived}, got {michi_id}"
                        )
                    else:
                        result["detail"] = f"{result['detail']}; michi_id coherent"
                except Exception as exc:
                    result["status"] = "fail"
                    result["detail"] = f"identity verification failed: {exc}"

        if result["status"] == "pass" and extract_vars and resp.ok:
            try:
                data = resp.json()
                for var_name, json_path in extract_vars.items():
                    value = _dotted_get(data, json_path)
                    if value is not None:
                        variables[f"var_{var_name}"] = value
            except Exception:
                pass

        return result

    except requests.exceptions.ConnectionError as e:
        return _fail(check_def["name"], f"Connection refused: {e}")
    except Exception as e:
        return _fail(check_def["name"], str(e))


def run_discovery_verify(runtime, check_def):
    """Signed discovery announce pipeline (contract section 2.2): schema,
    Ed25519 over the canonical bytes, michi_id coherence, +-90 s freshness
    and (michi_id, nonce) replay rejection, with negative cases."""
    name = check_def["name"]
    host = runtime.harness.base_host
    port = runtime.harness.base_port
    detail_parts = []
    try:
        verifier = DiscoveryVerifier()

        announce, _ = build_announce(host, port)
        schema_errors = runtime.schemas.validate("discovery-announce.schema.json", announce)
        if schema_errors:
            return _fail(name, f"dynamic announce fails the bundle schema: {schema_errors[0]}")
        detail_parts.append("dynamic announce: bundle schema valid")
        if derive_michi_id(b64url_decode(announce["public_key"])) != announce["michi_id"]:
            return _fail(name, "dynamic announce: michi_id not coherent with public_key")
        detail_parts.append("michi_id = base64url(BLAKE3(public_key)) coherent")
        if not ed25519_verify(announce["public_key"], announce["signature"], canonical_bytes(announce)):
            return _fail(name, "dynamic announce: Ed25519 signature does not verify")
        detail_parts.append("Ed25519 signature verifies over the canonical bytes")
        ok, reason = verifier.accept(announce)
        if not ok:
            return _fail(name, f"fresh signed announce REJECTED: {reason}")
        detail_parts.append(f"accepted ({reason})")

        ok, reason = verifier.accept(announce)
        if ok:
            return _fail(name, "replay of the same (michi_id, nonce) was ACCEPTED")
        detail_parts.append("replay of (michi_id, nonce) rejected")

        altered, _ = build_announce(host, port)
        altered["signature"] = corrupt_b64url_char(altered["signature"])
        ok, reason = verifier.accept(altered)
        if ok:
            return _fail(name, "announce with an altered signature was ACCEPTED")
        detail_parts.append("altered signature rejected")

        tampered, _ = build_announce(host, port)
        tampered["nonce"] = b64url_nopad(secrets.token_bytes(16))
        ok, reason = verifier.accept(tampered)
        if ok:
            return _fail(name, "announce with a tampered nonce was ACCEPTED")
        detail_parts.append("tampered nonce rejected (signature mismatch)")

        stale, _ = build_announce(host, port, now_ms=int(time.time() * 1000) - FRESHNESS_WINDOW_MS - 1000)
        ok, reason = verifier.accept(stale)
        if ok:
            return _fail(name, "stale announce was ACCEPTED")
        detail_parts.append("stale timestamp rejected (outside the +-90 s window)")

        mismatched, _ = build_announce(host, port)
        _, other_public_bytes = new_identity()
        mismatched["michi_id"] = derive_michi_id(other_public_bytes)
        ok, reason = verifier.accept(mismatched)
        if ok:
            return _fail(name, "announce with a mismatched michi_id was ACCEPTED")
        detail_parts.append("michi_id/public_key mismatch rejected")

        vector = json.loads((VECTORS_DIR / "announce-valid.json").read_text(encoding="utf-8"))
        vector_errors = runtime.schemas.validate("discovery-announce.schema.json", vector)
        if vector_errors:
            return _fail(name, f"bundle announce vector fails the schema: {vector_errors[0]}")
        if derive_michi_id(b64url_decode(vector["public_key"])) != vector["michi_id"]:
            return _fail(name, "bundle announce vector identity incoherent")
        if not ed25519_verify(vector["public_key"], vector["signature"], canonical_bytes(vector)):
            return _fail(name, "bundle announce vector signature does not verify")
        ok, reason = verifier.accept(vector)
        if ok:
            return _fail(name, "bundle announce vector with a stale timestamp was ACCEPTED")
        detail_parts.append("bundle vector: schema/signature/identity hold; freshness rejects the stale timestamp")

        return _pass(name, "; ".join(detail_parts))
    except Exception as exc:
        return _fail(name, str(exc))


def run_controller_identity(runtime, check_def):
    """Fresh Ed25519 controller identity + real challenge signature."""
    name = check_def["name"]
    variables = runtime.variables
    try:
        private_key, public_bytes = new_identity()
        nonce = secrets.token_bytes(16)
        signature = private_key.sign(nonce)
        public_key = b64url_nopad(public_bytes)
        michi_id = derive_michi_id(public_bytes)
        variables[f"var_{check_def.get('michi_id_var', 'controller_michi_id')}"] = michi_id
        variables[f"var_{check_def.get('public_key_var', 'controller_public_key')}"] = public_key
        variables[f"var_{check_def.get('nonce_var', 'controller_challenge_nonce')}"] = b64url_nopad(nonce)
        variables[f"var_{check_def.get('signature_var', 'controller_challenge_signature')}"] = b64url_nopad(signature)
        if not ed25519_verify(public_key, b64url_nopad(signature), nonce):
            return _fail(name, "the generated challenge signature does not verify")
        return _pass(
            name,
            f"fresh Ed25519 identity (michi_id={michi_id[:8]}...); "
            f"challenge signature verifies over the decoded nonce",
        )
    except Exception as exc:
        return _fail(name, str(exc))


def run_pairing_window_open(runtime, check_def):
    """The physical pairing press, via the simulator local channel."""
    name = check_def["name"]
    harness = runtime.harness
    if harness is None:
        return _fail(name, "no stream harness configured for this scenario")
    try:
        harness.open_pairing_window()
    except Exception as exc:
        return _fail(name, f"could not open the pairing window: {exc}")
    if harness.embedded:
        return _pass(
            name,
            "physical pairing window opened via the simulator internal hook "
            "(state.open_pairing_window)",
        )
    return _pass(
        name,
        "external simulator: the physical press is a device-side action "
        "(harness contract: start with --pairing-open); verified by the next pair/start",
    )


def run_read_local_pin(runtime, check_def):
    """Reads the dynamic PIN from the simulator local display channel."""
    name = check_def["name"]
    variables = runtime.variables
    harness = runtime.harness
    if harness is None:
        return _fail(name, "no stream harness configured for this scenario")
    try:
        session_id = render(check_def["session_id"], variables)
        pin = harness.read_local_pin(session_id, check_def.get("timeout_seconds", 10.0))
    except Exception as exc:
        return _fail(name, f"could not read the PIN from the local display channel: {exc}")
    variables[f"var_{check_def.get('pin_var', 'pairing_pin')}"] = pin
    return _pass(name, f"dynamic PIN read from the local display channel ({pin[0]}*****)")


def run_random_int(runtime, check_def):
    """Dynamic random integer bound to a variable."""
    name = check_def["name"]
    variables = runtime.variables
    low = int(check_def["min"])
    high = int(check_def["max"])
    value = low + secrets.randbelow(high - low + 1)
    variables[f"var_{check_def['var']}"] = value
    return _pass(name, f"{check_def['var']} = {value} (dynamic)")


def run_now_ms(runtime, check_def):
    """Dynamic epoch-milliseconds bound to a variable."""
    name = check_def["name"]
    variables = runtime.variables
    variables[f"var_{check_def.get('var', 'sent_at_ms')}"] = int(time.time() * 1000)
    return _pass(name, "dynamic epoch-milliseconds captured")


def run_udp_rtp_send(runtime, check_def):
    """Sends real RTP datagrams over UDP to the negotiated stream port."""
    name = check_def["name"]
    harness = runtime.harness
    if harness is None:
        return _fail(name, "no stream harness configured for this scenario")
    variables = runtime.variables
    try:
        packets = int(check_def.get("packets", 100))
        payload_type = int(check_def.get("payload_type", RTP_PAYLOAD_TYPE))
        payload_bytes = int(check_def.get("payload_bytes", RTP_PAYLOAD_BYTES))
        ssrc = render(check_def["ssrc"], variables)
        port = render(check_def["target_port"], variables)
        host = (
            render(check_def["target_host"], variables)
            if check_def.get("target_host")
            else harness.base_host
        )
        batch = int(check_def.get("batch", 20))

        sender = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        received = []
        try:
            if harness.embedded:
                sock = harness.state.stream_socket
                if sock is None:
                    return _fail(name, "the simulator has no active stream socket")
                sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 1 << 20)
                for offset in range(0, packets, batch):
                    for seq in range(offset + 1, min(offset + batch, packets) + 1):
                        sender.sendto(
                            build_packet(seq, seq * 480, ssrc, pcm10ms_payload(seq)),
                            (host, port),
                        )
                    received.extend(
                        harness.receive_stream_packets(min(batch, packets - offset))
                    )
            else:
                for seq in range(1, packets + 1):
                    sender.sendto(
                        build_packet(seq, seq * 480, ssrc, pcm10ms_payload(seq)),
                        (host, port),
                    )
        finally:
            sender.close()

        if not harness.embedded:
            return _pass(
                name,
                f"sent {packets} RTP datagrams (PT {payload_type}, SSRC {ssrc}, "
                f"{payload_bytes} B payload) over real UDP to {host}:{port} "
                f"(arrival is not observable in external mode)",
            )

        problems = []
        parsed = []
        for datagram in received:
            try:
                parsed.append(parse_packet(datagram))
            except ValueError as exc:
                problems.append(str(exc))
        if len(parsed) != packets:
            problems.append(f"received {len(parsed)}/{packets} datagrams")
        for packet_data in parsed:
            if packet_data["version_byte"] != 0x80:
                problems.append(
                    f"seq {packet_data['seq']}: not RTP v2 (0x{packet_data['version_byte']:02x})"
                )
            if packet_data["pt"] != payload_type:
                problems.append(f"seq {packet_data['seq']}: PT {packet_data['pt']} != {payload_type}")
            if packet_data["ssrc"] != ssrc:
                problems.append(f"seq {packet_data['seq']}: SSRC {packet_data['ssrc']} != {ssrc}")
            if packet_data["payload_len"] != payload_bytes:
                problems.append(
                    f"seq {packet_data['seq']}: payload {packet_data['payload_len']} B != {payload_bytes}"
                )
        seqs = sorted(packet_data["seq"] for packet_data in parsed)
        if seqs != list(range(1, packets + 1)):
            problems.append(f"sequence set is not exactly 1..{packets}")
        if problems:
            return _fail(name, f"{packets} RTP datagrams sent but arrivals are not canonical: {problems[0]}")
        return _pass(
            name,
            f"sent {packets} RTP datagrams (PT {payload_type}, SSRC {ssrc}, "
            f"{payload_bytes} B payload) over real UDP to {host}:{port}; "
            f"all {packets} arrived canonical at the bound socket",
        )
    except Exception as exc:
        return _fail(name, str(exc))


def run_rtp_guard_evidence(runtime, check_def):
    """Compiles and runs the SHARED firmware RTP guard host test (the same
    rtp_guard.c the receiver engine runs) as evidence for the source-IP/PT/
    SSRC/size rejection classes."""
    name = check_def["name"]
    try:
        sim_dir = Path(check_def.get("sim_dir") or default_sim_dir())
        sim_root = sim_dir.parent
        rtp_guard_c = sim_root / "firmware" / "components" / "michi_audio" / "rtp_guard.c"
        host_test_c = sim_root / "tests" / "host" / "test_rtp_guard.c"
        if not rtp_guard_c.exists() or not host_test_c.exists():
            return _fail(name, f"shared rtp_guard sources not found under {sim_root}")
        tmp = tempfile.mkdtemp(prefix="michi-rtp-guard-e2e-")
        binary = os.path.join(tmp, "test_rtp_guard")
        try:
            compile_proc = subprocess.run(
                [
                    "cc", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
                    "-D_DEFAULT_SOURCE",
                    f"-I{rtp_guard_c.parent}",
                    "-o", binary,
                    str(host_test_c),
                    str(rtp_guard_c),
                ],
                capture_output=True,
                text=True,
                timeout=120,
            )
            if compile_proc.returncode != 0:
                return _fail(name, f"rtp_guard host test did not compile: {compile_proc.stderr[:300]}")
            run_proc = subprocess.run([binary], capture_output=True, text=True, timeout=120)
            if run_proc.returncode != 0:
                return _fail(name, f"rtp_guard host test failed: {run_proc.stdout[-300:]}")
            output = run_proc.stdout
            for marker in ("classification per class", "all tests passed"):
                if marker not in output:
                    return _fail(name, f"rtp_guard evidence missing marker: {marker}")
            if "FAIL" in output:
                return _fail(name, "rtp_guard host test reported FAIL")
            return _pass(
                name,
                "shared firmware rtp_guard.c host test: source-IP/PT/SSRC/size "
                "rejection classes verified (the same source the firmware engine runs)",
            )
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    except Exception as exc:
        return _fail(name, str(exc))


def run_lease_expire(runtime, check_def):
    """Expires the session lease (clock advance embedded, real wait external)."""
    name = check_def["name"]
    harness = runtime.harness
    if harness is None:
        return _fail(name, "no stream harness configured for this scenario")
    seconds = check_def.get("seconds", 31)
    harness.advance_clock(seconds)
    if harness.embedded:
        return _pass(name, f"simulator mono clock advanced {seconds} s (lease expires without heartbeat)")
    return _pass(name, f"waited {seconds} s of real time for the lease to expire")


_STEP_RUNNERS = {
    "discovery_verify": run_discovery_verify,
    "controller_identity": run_controller_identity,
    "pairing_window_open": run_pairing_window_open,
    "read_local_pin": run_read_local_pin,
    "random_int": run_random_int,
    "now_ms": run_now_ms,
    "udp_rtp_send": run_udp_rtp_send,
    "rtp_guard_evidence": run_rtp_guard_evidence,
    "lease_expire": run_lease_expire,
}


def run_step(runtime, session, check_def):
    step_type = check_def.get("type", "http")
    if step_type == "http":
        return run_check(runtime, session, check_def)
    runner = _STEP_RUNNERS.get(step_type)
    if runner is None:
        return _fail(check_def["name"], f"Unsupported step type: {step_type}")
    return runner(runtime, check_def)


def bootstrap_harness(harness_cfg, args):
    mode = harness_cfg.get("mode", "auto")
    external = bool(args.stream_host and args.stream_port)
    if mode == "embedded":
        external = False
    elif mode == "external":
        external = True
    if external:
        return ExternalStreamHarness(args.stream_host, args.stream_port, args.stream_pin_log)
    sim_dir = args.stream_sim_dir or harness_cfg.get("sim_dir") or default_sim_dir()
    return EmbeddedStreamHarness(sim_dir)


def run_scenario(scenario_path, args):
    """Run a full scenario from a YAML file."""
    with open(scenario_path, "r") as f:
        scenario = yaml.safe_load(f)

    harness = None
    if scenario.get("harness"):
        harness = bootstrap_harness(scenario["harness"], args)

    runtime = ScenarioRuntime(args, harness)
    session = make_requests_session()
    report = ReportBuilder(
        scenario_id=scenario["id"],
        scenario_name=scenario["name"],
        server_type=scenario.get("server_type", "unknown"),
        client_type=scenario.get("client_type", "runner"),
        certifier=args.certified_by or os.environ.get("USER", "unknown"),
    )

    try:
        for check_def in scenario["checks"]:
            result = run_step(runtime, session, check_def)
            report.add_check(result["name"], result["status"], result["detail"])
            if result["status"] == "fail":
                print(f"  FAIL: {result['name']} — {result['detail']}")
                if args.fail_fast:
                    break
            else:
                print(f"  PASS: {result['name']} — {result['detail']}")
    finally:
        if harness is not None:
            harness.close()

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
    parser.add_argument("--stream-host", help="Stream simulator host (external mode)")
    parser.add_argument("--stream-port", type=int, help="Stream simulator port (external mode)")
    parser.add_argument("--stream-sim-dir", help="Canonical simulator directory (embedded mode)")
    parser.add_argument("--stream-pin-log", help="Simulator log file exposing [LOCAL DISPLAY] lines (external mode)")
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
