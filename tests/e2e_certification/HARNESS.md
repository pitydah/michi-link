# E2E Certification — Stream Simulator Harness Contract (E2E-09)

The `micro_stream_receiver.yml` scenario exercises the CANONICAL receiver
v1-lite contract (`contracts/receiver-v1-lite`) end to end against the
Michi Music Stream simulator (`michi-music-stream/simulator/receiver_sim.py`).

The simulator implements the contract exactly: the physical pairing window
is opened ONLY through the internal hook (`state.open_pairing_window`, the
same hook the CLI flag `--pairing-open` uses) and the 6-digit PIN is shown
ONLY on the local display channel (`[LOCAL DISPLAY]` log lines, enabled by
the dev-only `--show-local-pairing-pin` flag). Neither is ever exposed over
HTTP, and the simulator does not ingest RTP datagrams by design. That is
why the runner needs a LOCAL channel in both modes.

Nothing is hardcoded: the controller identity (Ed25519 keypair), the
challenge nonce/signature, the PIN, every session id, every token, the SSRC
and the stream port are generated at runtime.

## Modes

### Embedded (default)

```
python3 runner.py scenarios/micro_stream_receiver.yml
```

- imports the canonical simulator from the sibling checkout (auto-detected
  at `../michi-music-stream/simulator`; override with `--stream-sim-dir` or
  the `MICHI_STREAM_SIM_DIR` environment variable);
- serves it behind a REAL TCP HTTP server (werkzeug) on an ephemeral port;
- the physical pairing press is simulated through the internal hook (the
  same channel the simulator CLI flag uses);
- the PIN is read from the local display channel (captured
  `[LOCAL DISPLAY]` lines of the simulator logger);
- the simulator mono clock is injectable, so the 30 s session lease expiry
  is verified by advancing the clock (no real sleep);
- the bound RTP socket belongs to the harness: the 100 canonical RTP
  datagrams are sent over real UDP and every arrival is verified
  (RTP v2, PT 97, negotiated SSRC, 1920-byte payload, sequence 1..100).

### External (manually started simulator process)

```
python3 <michi-music-stream>/simulator/receiver_sim.py \
    --pairing-open --show-local-pairing-pin --port 8600 > /tmp/stream-sim.log 2>&1
python3 runner.py scenarios/micro_stream_receiver.yml \
    --stream-host 127.0.0.1 --stream-port 8600 --stream-pin-log /tmp/stream-sim.log
```

- the physical press happens at the device: the simulator MUST be started
  with `--pairing-open` (while the window is closed, `pair/start` returns
  the canonical 403 FORBIDDEN — verified by the simulator's own suite);
- `--stream-pin-log` points at the simulator log file exposing the
  `[LOCAL DISPLAY]` channel (requires `--show-local-pairing-pin`);
- the lease expiry step waits 31 s of real time (the external clock cannot
  be advanced);
- RTP arrival is not observable from outside the process: the step sends
  the 100 real datagrams to the negotiated port and the transport success
  is the evidence (the simulator binds the port but does not ingest RTP by
  design).

## Shared evidence (both modes)

The IP/PT/SSRC/size RTP rejection classes are evidenced by compiling and
running the SAME firmware guard the receiver engine runs
(`<michi-music-stream>/firmware/components/michi_audio/rtp_guard.c`,
host-tested by `tests/host/test_rtp_guard.c`). The step requires a C
compiler (`cc`). At the HTTP layer the scenario also proves the canonical
negotiation rejects: a `source_ip` override, a non-canonical payload type
and a zero SSRC (400 INVALID_REQUEST each).

## Discovery

The simulator does not multicast announces. The discovery step therefore
builds a freshly signed announce (Ed25519 keypair, nonce and timestamp all
generated at runtime) and runs the full controller-side verification
pipeline: bundle schema, Ed25519 over the canonical bytes, `michi_id =
base64url(BLAKE3(public_key))` coherence, the ±90 s freshness window and
`(michi_id, nonce)` replay rejection — plus the negative cases (altered
signature, tampered nonce, stale timestamp, mismatched identity). The
static bundle vector is verified for schema/signature/identity and is
correctly REJECTED by the freshness rule (its timestamp is from 2026-01-01).

## Dependencies

See `requirements.txt` (requests, PyYAML, jsonschema, referencing, blake3,
cryptography; flask + werkzeug for the embedded harness).
