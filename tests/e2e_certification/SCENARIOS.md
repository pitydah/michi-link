# Michi Link E2E Certification — Scenario Definitions

## Scenario: mobile_player_pairing

**ID:** E2E-01
**File:** `mobile_player.yml`
**Server:** Player (port 8400)
**Client:** Mobile

### Checks

1. Discovery — UDP announce received
2. GET /api/v1/server/info → service: michi-music-player, auth.strategy: PLAYER_PASSWORD
3. POST /api/v1/pair/start → pairing_code received
4. POST /api/v1/pair/confirm → Bearer token received
5. GET /api/v1/tracks?limit=5 with token → 200 OK
6. GET /api/v1/tracks?limit=5 without token → 401 UNAUTHORIZED

### Run command
```
python runner.py scenarios/mobile_player.yml --server-host 192.168.1.100 --server-port 8400
```

---

## Scenario: mobile_micro_pairing

**ID:** E2E-04
**File:** `mobile_micro.yml`
**Server:** Micro Server (port 8500)
**Client:** Mobile

### Checks

1. GET /api/v1/server/info → service: michi-micro-server, auth.strategy: SERVER_CODE
2. POST /api/v1/pair/start → pairing_code + expires_at
3. POST /api/v1/pair/confirm → device_token + refresh_token + permissions
4. POST /api/v1/token/refresh → new device_token + refresh_token
5. GET /api/v1/tracks?limit=5 with token → 200 OK

### Run command
```
python runner.py scenarios/mobile_micro.yml --server-host 192.168.1.101 --server-port 8500
```

---

## Scenario: mobile_micro_download

**ID:** E2E-05
**File:** `mobile_micro.yml`
**Server:** Micro Server (port 8500)
**Client:** Mobile

### Checks

1. GET /api/v1/tracks?limit=1 → get first track_id
2. GET /api/v1/download/{track_id} → 200 OK, Content-Disposition: attachment
3. GET /api/v1/sync/manifest → cursor + tracks
4. GET /api/v1/sync/manifest/delta?cursor=0 → cursor + added

### Run command
```
python runner.py scenarios/mobile_micro.yml --server-host 192.168.1.101 --server-port 8500 --token <token>
```

---

## Scenario: mobile_player_playback

**ID:** E2E-03
**File:** `mobile_player.yml`
**Server:** Player (port 8400)
**Client:** Mobile

### Checks

1. GET /api/v1/playback/state → state, track_id, position_ms, volume
2. POST /api/v1/playback/control {"command": "play"} → status ok
3. POST /api/v1/playback/control {"command": "seek", "position_ms": 30000} → status ok
4. POST /api/v1/playback/control {"command": "set_volume", "volume": 50} → status ok
5. POST /api/v1/playback/control {"command": "pause"} → status ok
6. GET /api/v1/playback/state → state: paused

### Run command
```
python runner.py scenarios/mobile_player.yml --server-host 192.168.1.100 --server-port 8400 --token <token>
```

---

## Scenario: player_micro_import

**ID:** E2E-07
**File:** `player_micro_import.yml`
**Server:** Micro Server (port 8500)
**Client:** Player

### Checks

1. Pairing with Micro Server (PLAYER_PASSWORD on Player side, SERVER_CODE on Micro)
2. POST /api/v1/import/session → session_id
3. POST /api/v1/import/upload/{session_id} → track_id (repeat 3x)
4. POST /api/v1/import/commit/{session_id} → tracks_imported: 3
5. GET /api/v1/library/stats → total_tracks >= 3

### Run command
```
python runner.py scenarios/player_micro_import.yml --player-host 192.168.1.100 --micro-host 192.168.1.101
```

---

## Scenario: micro_autonomous_playback

**ID:** E2E-08
**File:** `micro_autonomous_playback.yml`
**Server:** Micro Server (port 8500)
**Client:** (standalone)

### Checks

1. POST /api/v1/playback/control {"command": "play"} → status ok
2. GET /api/v1/playback/state → state: playing, track_id present
3. POST /api/v1/queue/items {"track_ids": ["..."], "position": "next"} → items_added
4. POST /api/v1/playback/control {"command": "seek", "position_ms": 60000} → status ok
5. POST /api/v1/playback/control {"command": "stop"} → status ok
6. GET /api/v1/playback/state → state: stopped

### Run command
```
python runner.py scenarios/micro_autonomous_playback.yml --server-host 192.168.1.101 --server-port 8500 --token <token>
```

---

## Scenario: micro_stream_receiver

**ID:** E2E-09
**File:** `micro_stream_receiver.yml`
**Server:** Michi Music Stream simulator (canonical receiver v1-lite)
**Client:** Michi Micro Server (controller)

### Checks

1. Signed discovery announce: bundle schema, Ed25519 over the canonical
   bytes, michi_id = base64url(BLAKE3(public_key)), ±90 s freshness and
   (michi_id, nonce) replay rejection; altered signature / tampered nonce /
   stale timestamp / mismatched identity rejected; the static bundle vector
   is rejected by the freshness rule.
2. GET /api/v1/server/info → canonical profile + identity coherence.
3. Physical pairing window opened via the simulator internal hook (local
   channel — never over HTTP).
4. POST /api/v1/pair/start with a real Ed25519 challenge (dynamic identity).
5. GET /api/v1/pair/status?session_id=... → pending.
6. Dynamic PIN read from the simulator local display channel.
7. POST /api/v1/pair/confirm → receiver-issued Bearer token; replay → 409
   (the pairing session is single-use).
8. POST /api/v1/receiver-lite/session → 201; UDP port 49152–65535.
9. IP/PT/SSRC negotiation rejects (400) + shared firmware rtp_guard.c host
   test for the RTP rejection classes (source IP / PT / SSRC / size).
10. 100 canonical RTP packets (PT 97, negotiated SSRC, 1920 B) over real UDP.
11. POST /api/v1/receiver-lite/heartbeat → alive; replay → 409; the next
    sequence renews the lease.
12. PATCH /api/v1/receiver-lite/session → volume 55; pause; resume.
13. GET /api/v1/receiver-lite/session without exposing session_token.
14. DELETE /api/v1/receiver-lite/session → 204; GET → 404; new session → 201.
15. Lease expiry (clock advanced in embedded mode; real 31 s externally).
16. Retired legacy receiver routes → canonical 404 NOT_FOUND (negative).

### Run commands
```
# Embedded harness (default: imports the canonical simulator, opens the
# physical pairing window via the internal hook, real TCP + real UDP):
python runner.py scenarios/micro_stream_receiver.yml

# External simulator process:
python3 <michi-music-stream>/simulator/receiver_sim.py --pairing-open --show-local-pairing-pin --port 8600 > /tmp/stream-sim.log 2>&1
python runner.py scenarios/micro_stream_receiver.yml --stream-host 127.0.0.1 --stream-port 8600 --stream-pin-log /tmp/stream-sim.log
```

Harness contract: see `HARNESS.md`.
