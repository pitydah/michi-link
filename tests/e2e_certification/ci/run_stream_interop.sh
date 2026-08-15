#!/usr/bin/env bash
#
# Cross-repository stream interoperability job (P1-03).
#
# This script is the SINGLE source of truth for the job: the CI workflow
# (.github/workflows/stream-interop.yml) and local executions run exactly the
# same commands through it.
#
#   1.  Michi Music Stream is cloned and checked out PINNED to an exact
#       commit SHA. A mutable checkout is refused: the resolved HEAD must
#       equal the expected SHA or the job fails.
#   2.  Bundle verification: the Link normative bundle
#       (contracts/receiver-v1-lite) must be byte-identical to the vendored
#       Stream copy (contracts/michi-link), including the manifest-internal
#       sha256 hashes; any drift fails the job.
#   3.  The canonical receiver simulator is started on a DYNAMIC port with
#       --pairing-open and the local PIN channel (--show-local-pairing-pin).
#   4.  Readiness probe with a bounded timeout.
#   5.  The canonical E2E-09 scenario runs from Link (runner in external
#       mode) against the running simulator.
#   6.  The simulator is stopped ALWAYS (trap/teardown), even on failure.
#   7.  Logs are sanitized (PIN masked, tokens redacted, the raw log deleted)
#       and collected as CI artifacts.
#   8.  Legacy route scan: positive references fail the job.
#   9.  A byte-deterministic certification report in the exact mission shape
#       (no timestamps) is generated and published as a CI artifact.
#
# This job certifies the canonical receiver SIMULATOR only. It never asserts
# DEVICE_E2E_PASS / NETWORK_E2E_PASS / hardware certified / production ready.
#
# Parameters (env, all optional):
#   STREAM_REPO_URL            Stream origin (https URL in CI; a local path is
#                              accepted for the local demonstration of this
#                              job, e.g. /home/cristian/michi-music-stream)
#   STREAM_REF                 Stream branch/ref under test
#   STREAM_SHA                 EXACT commit the checkout must resolve to
#   STREAM_DEFAULT_BRANCH_SHA  expected SHA of the Stream default branch tip
#                              (used only when STREAM_SHA is empty)
#   STREAM_CHECKOUT_DIR        where Stream is cloned (must not pre-exist)
#   ARTIFACT_DIR               sanitized logs + report copies
#   REPORT_PATH                canonical deterministic report destination
#   SIM_READY_TIMEOUT_S        readiness timeout in seconds (default 30)

set -euo pipefail

LINK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$LINK_ROOT"

STREAM_REPO_URL="${STREAM_REPO_URL:-https://github.com/pitydah/michi-music-stream.git}"
STREAM_REF="${STREAM_REF:-fix/runtime-discovery-crossrepo-hardening}"
STREAM_SHA="${STREAM_SHA:-}"
STREAM_DEFAULT_BRANCH_SHA="${STREAM_DEFAULT_BRANCH_SHA:-}"
STREAM_CHECKOUT_DIR="${STREAM_CHECKOUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/michi-stream-checkout.XXXXXX")}"
ARTIFACT_DIR="${ARTIFACT_DIR:-$LINK_ROOT/tests/e2e_certification/ci-artifacts}"
REPORT_PATH="${REPORT_PATH:-$LINK_ROOT/tests/e2e_certification/results/stream-interop-alpha1.json}"
SIM_READY_TIMEOUT_S="${SIM_READY_TIMEOUT_S:-30}"

BUNDLE_LINK="$LINK_ROOT/contracts/receiver-v1-lite"
SCENARIO="$LINK_ROOT/tests/e2e_certification/scenarios/micro_stream_receiver.yml"
RUNNER="$LINK_ROOT/tests/e2e_certification/runner.py"
CI_DIR="$LINK_ROOT/tests/e2e_certification/ci"

SIM_PID=""
RAW_LOG=""
SIM_HOST="127.0.0.1"
SIM_PORT=""

log() { printf '[stream-interop] %s\n' "$*"; }

sanitize_log() {
  sed -E \
    -e 's/(Pairing PIN: )[0-9]{6}/\1******/g' \
    -e 's/(pairing token: ).*/\1<redacted>/g' \
    -e 's/(session token: ).*/\1<redacted>/g' \
    "$1"
}

fail() {
  log "ERROR: $*"
  if [[ -n "$RAW_LOG" && -f "$RAW_LOG" ]]; then
    log "simulator log tail (sanitized):"
    sanitize_log "$RAW_LOG" | tail -n 20 >&2 || true
  fi
  exit 1
}

teardown() {
  if [[ -n "$SIM_PID" ]] && kill -0 "$SIM_PID" 2>/dev/null; then
    log "stopping the simulator (pid $SIM_PID)..."
    kill -TERM "$SIM_PID" 2>/dev/null || true
    for _ in $(seq 1 10); do
      kill -0 "$SIM_PID" 2>/dev/null || break
      sleep 0.3
    done
    if kill -0 "$SIM_PID" 2>/dev/null; then
      log "simulator did not exit after SIGTERM; sending SIGKILL"
      kill -KILL "$SIM_PID" 2>/dev/null || true
    fi
    wait "$SIM_PID" 2>/dev/null || true
    log "simulator stopped"
  fi
  if [[ -n "$RAW_LOG" && -f "$RAW_LOG" ]]; then
    sanitize_log "$RAW_LOG" > "$ARTIFACT_DIR/simulator.log"
    rm -f "$RAW_LOG"
    log "sanitized simulator log saved to $ARTIFACT_DIR/simulator.log (raw log deleted)"
  fi
}

# Fresh artifact dir (safety guard against destructive rm -rf).
if [[ -z "$ARTIFACT_DIR" || "$ARTIFACT_DIR" = "/" || "$ARTIFACT_DIR" = "." || "$ARTIFACT_DIR" = "$HOME" ]]; then
  fail "refusing to clean unsafe ARTIFACT_DIR: $ARTIFACT_DIR"
fi
rm -rf "$ARTIFACT_DIR"
mkdir -p "$ARTIFACT_DIR"
trap teardown EXIT

# 0. Link identity: the report records the exact commit under test.
if ! git rev-parse --git-dir >/dev/null 2>&1; then
  fail "LINK_ROOT is not a git repository: $LINK_ROOT"
fi
LINK_SHA="$(git rev-parse HEAD)"
log "Michi Link commit under test: $LINK_SHA"

# 1. Pinned Stream checkout (mutable checkouts are refused).
if [[ -z "$STREAM_SHA" && -z "$STREAM_DEFAULT_BRANCH_SHA" ]]; then
  fail "no pinned Stream SHA: set STREAM_SHA (exact commit) or STREAM_DEFAULT_BRANCH_SHA (expected default branch tip)"
fi
if [[ -n "$STREAM_SHA" && -n "$STREAM_DEFAULT_BRANCH_SHA" ]]; then
  fail "set EITHER STREAM_SHA or STREAM_DEFAULT_BRANCH_SHA, not both"
fi

if [[ -z "$STREAM_CHECKOUT_DIR" || "$STREAM_CHECKOUT_DIR" = "/" || "$STREAM_CHECKOUT_DIR" = "." || "$STREAM_CHECKOUT_DIR" = "$HOME" ]]; then
  fail "refusing to clean unsafe STREAM_CHECKOUT_DIR: $STREAM_CHECKOUT_DIR"
fi
rm -rf "$STREAM_CHECKOUT_DIR"

log "cloning Michi Music Stream from $STREAM_REPO_URL (no checkout yet)"
git clone --no-checkout -q "$STREAM_REPO_URL" "$STREAM_CHECKOUT_DIR" \
  || fail "could not clone $STREAM_REPO_URL"

EXPECTED_STREAM_SHA=""
if [[ -n "$STREAM_SHA" ]]; then
  EXPECTED_STREAM_SHA="$STREAM_SHA"
  log "fetching ref $STREAM_REF to resolve pinned SHA $STREAM_SHA"
  git -C "$STREAM_CHECKOUT_DIR" fetch -q origin "$STREAM_REF" \
    || fail "cannot fetch stream ref '$STREAM_REF' — is the Stream hardening branch pushed? (a mutable ref is never checked out)"
  git -C "$STREAM_CHECKOUT_DIR" checkout -q --detach "$STREAM_SHA" \
    || fail "commit $STREAM_SHA is not available from ref $STREAM_REF"
else
  EXPECTED_STREAM_SHA="$STREAM_DEFAULT_BRANCH_SHA"
  log "no explicit STREAM_SHA: pinning the default branch to $STREAM_DEFAULT_BRANCH_SHA"
  git -C "$STREAM_CHECKOUT_DIR" fetch -q origin "$STREAM_REF" \
    || fail "cannot fetch stream ref '$STREAM_REF'"
  git -C "$STREAM_CHECKOUT_DIR" checkout -q --detach "$STREAM_DEFAULT_BRANCH_SHA" \
    || fail "commit $STREAM_DEFAULT_BRANCH_SHA is not available from ref $STREAM_REF"
fi

RESOLVED_STREAM_SHA="$(git -C "$STREAM_CHECKOUT_DIR" rev-parse HEAD)"
if [[ "$RESOLVED_STREAM_SHA" != "$EXPECTED_STREAM_SHA" ]]; then
  fail "checkout resolved to $RESOLVED_STREAM_SHA but the expected pinned SHA is $EXPECTED_STREAM_SHA"
fi
log "Michi Music Stream checkout resolved to the exact pinned SHA: $RESOLVED_STREAM_SHA (ref: $STREAM_REF)"

# 2. Bundle verification (Link normative bundle vs vendored Stream copy).
python3 "$CI_DIR/verify_bundle.py" "$BUNDLE_LINK" "$STREAM_CHECKOUT_DIR/contracts/michi-link" \
  || fail "the Stream vendored bundle derives from the normative Link bundle"

# 3. Dynamic port for the canonical receiver simulator.
SIM_PORT="$(python3 - <<'PY'
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
log "simulator dynamic port: $SIM_PORT"

# 4. Start the canonical simulator (pairing window open + local PIN channel).
RAW_LOG="$(mktemp "${TMPDIR:-/tmp}/stream-sim-raw.XXXXXX.log")"
log "starting receiver_sim.py (raw log: $RAW_LOG)"
python3 "$STREAM_CHECKOUT_DIR/simulator/receiver_sim.py" \
  --host "$SIM_HOST" --port "$SIM_PORT" \
  --pairing-open --show-local-pairing-pin \
  >"$RAW_LOG" 2>&1 &
SIM_PID=$!
log "simulator pid: $SIM_PID"

# 5. Readiness probe with a bounded timeout.
ready=0
for _ in $(seq 1 "$(( SIM_READY_TIMEOUT_S * 2 ))"); do
  if curl -fsS --max-time 2 "http://$SIM_HOST:$SIM_PORT/api/v1/server/info" >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$SIM_PID" 2>/dev/null; then
    fail "the simulator process exited before becoming ready"
  fi
  sleep 0.5
done
if [[ "$ready" != "1" ]]; then
  fail "the simulator did not become ready within ${SIM_READY_TIMEOUT_S}s"
fi
log "simulator ready at http://$SIM_HOST:$SIM_PORT"

# 6. Canonical E2E-09 scenario from Link (runner in external mode). The
#    simulator under test is already running; the raw log exposes the local
#    PIN channel. set -o pipefail propagates the runner exit code through tee.
log "running the canonical scenario (runner, external mode)"
{
  MICHI_STREAM_SIM_DIR="$STREAM_CHECKOUT_DIR/simulator" \
  python3 "$RUNNER" "$SCENARIO" \
    --stream-host "$SIM_HOST" --stream-port "$SIM_PORT" \
    --stream-pin-log "$RAW_LOG" \
    --certified-by ci-stream-interop
} 2>&1 | tee "$ARTIFACT_DIR/runner-console.log"

# 7. Legacy route scan (positive references fail the job).
python3 "$CI_DIR/legacy_scan.py" "$LINK_ROOT" \
  || fail "legacy route references found outside the canonical negative checks"

# 8. Deterministic certification report (exact mission shape, no timestamps).
python3 "$CI_DIR/generate_report.py" \
  --link-commit "$LINK_SHA" \
  --stream-commit "$RESOLVED_STREAM_SHA" \
  --out "$REPORT_PATH" >/dev/null
REGEN_TMP="$(mktemp "${TMPDIR:-/tmp}/stream-interop-regen.XXXXXX.json")"
python3 "$CI_DIR/generate_report.py" \
  --link-commit "$LINK_SHA" \
  --stream-commit "$RESOLVED_STREAM_SHA" \
  --out "$REGEN_TMP" >/dev/null
if ! cmp -s "$REGEN_TMP" "$REPORT_PATH"; then
  rm -f "$REGEN_TMP"
  fail "the certification report is not byte-deterministic"
fi
rm -f "$REGEN_TMP"
log "certification report generated (byte-deterministic): $REPORT_PATH"

# 9. Publish the report and the sanitized logs as CI artifacts.
cp "$REPORT_PATH" "$ARTIFACT_DIR/stream-interop-alpha1.json"
RUNNER_REPORT="$(ls -1t "$LINK_ROOT/tests/e2e_certification/reports/"micro_stream_receiver-*.json 2>/dev/null | head -n 1 || true)"
if [[ -n "$RUNNER_REPORT" ]]; then
  cp "$RUNNER_REPORT" "$ARTIFACT_DIR/runner-report.json"
fi

if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  {
    echo "## Stream interoperability (canonical receiver simulator)"
    echo "- certification: michi-link-alpha1 (MOCK_PASS)"
    echo "- michi_link commit: $LINK_SHA"
    echo "- michi_music_stream commit: $RESOLVED_STREAM_SHA (ref: $STREAM_REF)"
    echo "- environment: canonical receiver simulator"
    echo "- device_e2e: false"
    echo "- report: \`tests/e2e_certification/results/stream-interop-alpha1.json\`"
  } >> "$GITHUB_STEP_SUMMARY"
fi

log "SUCCESS: canonical scenario passed against the pinned Stream checkout"
log "  michi_link commit: $LINK_SHA"
log "  michi_music_stream commit: $RESOLVED_STREAM_SHA"
log "  report: $REPORT_PATH"
log "  artifacts: $ARTIFACT_DIR"
