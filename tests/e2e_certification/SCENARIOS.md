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
**Server:** Micro Server (port 8500) + Stream Simulator
**Client:** Stream (simulated)

### Checks

1. Stream announces via UDP → Micro receives
2. POST /receiver/pair/start → pairing_code
3. POST /receiver/pair/confirm → device_token
4. POST /receiver/heartbeat → server_time
5. POST /receiver/session/start with stream_url → session_id
6. POST /receiver/volume {"volume": 60} → volume: 60
7. POST /receiver/session/stop → previous_state

### Run command
```
python runner.py scenarios/micro_stream_receiver.yml --micro-host 192.168.1.101 --micro-port 8500
```
