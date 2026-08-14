"""Stream simulator harness for the E2E certification runner.

Two modes, one contract (documented in HARNESS.md):

  embedded (default) — imports the canonical receiver simulator
  (receiver_sim.py) from the michi-music-stream checkout, serves it behind a
  REAL TCP HTTP server (werkzeug) on an ephemeral port and provides the
  simulator's LOCAL channels:

    * the physical pairing window is opened through the internal hook
      (state.open_pairing_window — the same hook the simulator CLI flag
      --pairing-open uses);
    * the PIN is read from the local DISPLAY channel ([LOCAL DISPLAY] log
      lines captured from the simulator logger — the same lines the
      dev-only --show-local-pairing-pin flag prints);
    * the simulator mono clock is injectable (lease expiry without a real
      31 s wait);
    * the bound RTP socket belongs to the harness, so real UDP datagrams
      sent to the negotiated port can be verified packet by packet.

  external — talks to a manually started simulator process (base URL + a
  log file exposing the [LOCAL DISPLAY] channel). The physical pairing
  window must be opened at the device (start the simulator with
  --pairing-open); RTP arrivals are not observable from outside the
  process (the simulator binds the port but does not ingest RTP by design).

The canonical simulator never exposes the pairing window or the PIN over
HTTP — that is why the harness needs a local channel in both modes.
"""

import logging
import os
import re
import socket
import sys
import threading
import time
from pathlib import Path

PIN_LINE_RE = re.compile(r"\[LOCAL DISPLAY\] Pairing PIN: ([0-9]{6})")
SESSION_LINE_RE = re.compile(
    r"\[LOCAL DISPLAY\] Pairing session: "
    r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})"
)


def default_sim_dir():
    """Auto-detect the canonical simulator directory of the sibling checkout."""
    candidates = []
    env = os.environ.get("MICHI_STREAM_SIM_DIR")
    if env:
        candidates.append(Path(env))
    michi_link_root = Path(__file__).resolve().parents[2]
    candidates.append(michi_link_root.parent / "michi-music-stream" / "simulator")
    candidates.append(Path.home() / "michi-music-stream" / "simulator")
    for candidate in candidates:
        if (Path(candidate) / "receiver_sim.py").exists():
            return str(candidate)
    raise FileNotFoundError(
        "canonical receiver simulator not found; set MICHI_STREAM_SIM_DIR or "
        "pass --stream-sim-dir"
    )


def _parse_display_lines(lines):
    """Parses [LOCAL DISPLAY] lines; returns (pin, session_id) of the latest
    pairing display, or (None, None)."""
    pin = None
    session_id = None
    for line in lines:
        match = PIN_LINE_RE.search(line)
        if match:
            pin = match.group(1)
        match = SESSION_LINE_RE.search(line)
        if match:
            session_id = match.group(1)
    return pin, session_id


class FakeMonoClock:
    """Injectable monotonic clock (the simulator lease/watchdog clock)."""

    def __init__(self, start=0.0):
        self.now = float(start)

    def __call__(self):
        return self.now

    def advance(self, seconds):
        self.now += seconds


class DisplayLogCapture(logging.Handler):
    """Captures the simulator's local display channel ([LOCAL DISPLAY] lines)."""

    def __init__(self):
        super().__init__(level=logging.INFO)
        self.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
        self._lines = []
        self._lock = threading.Lock()

    def emit(self, record):
        try:
            line = self.format(record)
        except Exception:
            line = record.getMessage()
        with self._lock:
            self._lines.append(line)

    def text(self):
        with self._lock:
            return list(self._lines)

    def wait_for_pairing(self, session_id, timeout_s=10.0):
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            pin, displayed_session = _parse_display_lines(self.text())
            if displayed_session == session_id:
                if pin is None:
                    raise RuntimeError(
                        "a [LOCAL DISPLAY] session line appeared without a PIN line"
                    )
                return pin
            time.sleep(0.1)
        raise RuntimeError(
            f"timed out waiting for the [LOCAL DISPLAY] PIN of pairing session {session_id}"
        )


class EmbeddedStreamHarness:
    """Canonical simulator in-process behind a REAL TCP HTTP server."""

    embedded = True

    def __init__(self, sim_dir):
        sim_dir = Path(sim_dir).resolve()
        if not (sim_dir / "receiver_sim.py").exists():
            raise FileNotFoundError(f"receiver_sim.py not found under {sim_dir}")
        if str(sim_dir) not in sys.path:
            sys.path.insert(0, str(sim_dir))

        import receiver_sim as sim_module  # noqa: F401

        from werkzeug.serving import make_server

        self.sim_module = sim_module
        self.clock = FakeMonoClock()
        self.state = sim_module.SimulatorState(
            sim_module.STANDARD_CONFIG,
            mono_clock=self.clock,
            show_local_pin=True,
        )
        self.display = DisplayLogCapture()
        sim_module.log.addHandler(self.display)
        self.app = sim_module.create_app(self.state)
        self.server = make_server("127.0.0.1", 0, self.app, threaded=True)
        self.port = self.server.server_port
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    base_host = "127.0.0.1"

    @property
    def base_port(self):
        return self.port

    def open_pairing_window(self):
        self.state.open_pairing_window()

    def read_local_pin(self, session_id, timeout_s=10.0):
        return self.display.wait_for_pairing(session_id, timeout_s)

    def advance_clock(self, seconds):
        self.clock.advance(seconds)

    def receive_stream_packets(self, count, timeout_s=10.0):
        sock = self.state.stream_socket
        if sock is None:
            raise RuntimeError("the simulator has no active stream socket")
        sock.settimeout(timeout_s)
        packets = []
        deadline = time.monotonic() + timeout_s
        while len(packets) < count and time.monotonic() < deadline:
            try:
                datagram, _addr = sock.recvfrom(65536)
            except socket.timeout:
                break
            packets.append(datagram)
        if len(packets) != count:
            raise RuntimeError(
                f"received {len(packets)}/{count} RTP datagrams at the bound socket"
            )
        return packets

    def close(self):
        try:
            self.server.shutdown()
        except Exception:
            pass
        self.thread.join(timeout=5)
        sim_log = getattr(self.sim_module, "log", None)
        if sim_log is not None:
            sim_log.removeHandler(self.display)


class ExternalStreamHarness:
    """Talks to a manually started simulator process."""

    embedded = False

    def __init__(self, host, port, pin_log_path=None):
        self.base_host = host
        self.base_port = port
        self.pin_log = Path(pin_log_path) if pin_log_path else None

    def open_pairing_window(self):
        # The physical press is a device-side action: the simulator must have
        # been started with --pairing-open (see HARNESS.md). Verified by the
        # next pair/start (403 while the window is closed).
        return None

    def read_local_pin(self, session_id, timeout_s=10.0):
        if not self.pin_log:
            raise RuntimeError(
                "no PIN channel: pass --stream-pin-log pointing at the simulator "
                "log file (requires --show-local-pairing-pin)"
            )
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            try:
                lines = self.pin_log.read_text(encoding="utf-8").splitlines()
            except OSError:
                lines = []
            pin, displayed_session = _parse_display_lines(lines)
            if displayed_session == session_id:
                if pin is None:
                    raise RuntimeError(
                        "a [LOCAL DISPLAY] session line appeared without a PIN line"
                    )
                return pin
            time.sleep(0.1)
        raise RuntimeError(
            f"timed out waiting for the [LOCAL DISPLAY] PIN of pairing session {session_id}"
        )

    def advance_clock(self, seconds):
        time.sleep(seconds)

    def receive_stream_packets(self, count, timeout_s=10.0):
        raise RuntimeError("RTP arrivals are not observable from outside the simulator process")

    def close(self):
        pass
