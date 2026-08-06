# E2E Test Plan — Michi Link API v1.0.0-alpha

> Los IDs de escenario siguen `tests/e2e_certification/SCENARIOS.md` (fuente): E2E-01, E2E-03, E2E-04, E2E-05, E2E-07, E2E-08, E2E-09. Los IDs E2E-02, E2E-06 y E2E-10 NO existen.

## Escenario A: Mobile ↔ Player (E2E-01, E2E-03)

**Objetivo:** Verificar que Michi Music Mobile puede descubrir, emparejar, explorar y controlar Michi Music Player.

**Server:** Player (port 8400) — ver `tests/e2e_certification/SCENARIOS.md`.

### A.1 Discovery

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Mobile escanea red vía UDP multicast (`224.0.0.167:53318`, announce firmado) | Recibe announce de Player |
| 2 | Mobile consulta `GET /api/v1/server/info` del Player | Recibe service: `michi-music-player`, auth.strategy: `PLAYER_PASSWORD` |

### A.2 Pairing (PLAYER_PASSWORD)

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Mobile envía `POST /api/v1/pair/start` con identidad (`michi_id`/`public_key`) + challenge Ed25519 (`challenge_nonce`/`challenge_signature`) | Recibe `session_id` + `expires_at` + identidad del servidor |
| 2 | Usuario lee el PIN de 6 dígitos en la pantalla del Player | — |
| 3 | Mobile envía `POST /api/v1/pair/confirm` con `session_id` + `pin` | Recibe `token` (Bearer) |
| 4 | Mobile usa token en header `Authorization: Bearer <token>` | Acceso concedido |

### A.3 Library Browse

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/tracks?limit=10` | Lista de 10 tracks con id, title, artist, album, duration_ms |
| 2 | `GET /api/v1/tracks/{id}` | Track completo |
| 3 | `GET /api/v1/albums?limit=5` | Lista de álbumes |
| 4 | `GET /api/v1/search?q=query` | Resultados en tracks, albums, artists |

### A.4 Streaming & Artwork

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/stream/{track_id}` con header `Range: bytes=0-1023` | 206 Partial Content, Content-Range header |
| 2 | `GET /api/v1/stream/{track_id}` sin Range | 200 OK, Content-Type audio/* |
| 3 | `GET /api/v1/artwork/{cover_id}` | 200 OK, Content-Type image/* |

### A.5 Playback Control

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/playback/state` | state, track_id, position_ms, volume |
| 2 | `POST /api/v1/playback/control` con `{ "command": "play" }` | status ok, Player reproduce |
| 3 | `POST /api/v1/playback/control` con `{ "command": "pause" }` | status ok, Player pausa |
| 4 | `POST /api/v1/playback/control` con `{ "command": "seek", "position_ms": 60000 }` | status ok, Player salta a 60s |
| 5 | `POST /api/v1/playback/control` con `{ "command": "set_volume", "volume": 50 }` | status ok, volumen 50% |
| 6 | `POST /api/v1/playback/control` con `{ "command": "next" }` | status ok, siguiente track |
| 7 | `POST /api/v1/playback/control` con `{ "command": "previous" }` | status ok, track anterior |

### A.6 Queue

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/queue` | Cola actual con items[], current_index |
| 2 | `POST /api/v1/queue/items` con `{ "track_ids": [...], "position": "next" }` | items_added > 0 |
| 3 | `POST /api/v1/queue/jump` con `{ "queue_item_id": "..." }` | current_index actualizado |

---

## Escenario B: Mobile ↔ Micro Server (E2E-04, E2E-05)

**Objetivo:** Verificar que Mobile puede emparejar, sincronizar, descargar y controlar Michi Micro Server.

**Server:** Micro Server (port 8500) — ver `tests/e2e_certification/SCENARIOS.md`.

### B.1 Discovery

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Mobile descubre Micro Server vía mDNS/UDP | Recibe announce |
| 2 | `GET /api/v1/server/info` | service: `michi-micro-server`, auth.strategy: `SERVER_CODE`, token_refresh: true |

### B.2 Pairing (SERVER_CODE)

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `POST /api/v1/pair/start` con identidad + challenge Ed25519 | Recibe session_id + expires_at |
| 2 | Usuario lee el PIN de 6 dígitos en la UI del Micro Server | — |
| 3 | `POST /api/v1/pair/confirm` con session_id + pin | Recibe device_token + refresh_token |

### B.3 Token Refresh

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `POST /api/v1/token/refresh` con refresh_token | Nuevo device_token + refresh_token |
| 2 | Token anterior deja de funcionar | 401 UNAUTHORIZED |

### B.4 Library & Download

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/tracks` | Lista de tracks |
| 2 | `GET /api/v1/download/{track_id}` | 200 OK, Content-Disposition: attachment, archivo completo |
| 3 | `GET /api/v1/artwork/{cover_id}` | 200 OK, imagen |

### B.5 Sync

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/sync/manifest` | cursor + tracks[] |
| 2 | `GET /api/v1/sync/manifest/delta?cursor=0&device_id=...` | cursor + added[] + deleted[] |
| 3 | `POST /api/v1/sync/state` con `{ "device_id": ..., "downloaded_tracks": [...] }` | status ok |

### B.6 Playback Control

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/playback/state` | state, track_id, position_ms, volume |
| 2 | `POST /api/v1/playback/control` con `{ "command": "play" }` | Micro Server reproduce |
| 3 | `POST /api/v1/playback/control` con `{ "command": "set_volume", "volume": 80 }` | Volumen 80% |
| 4 | `POST /api/v1/queue/items` con `{ "track_ids": [...] }` | items agregados |
| 5 | `GET /api/v1/queue` | Cola actualizada |

### B.7 Error Handling

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `GET /api/v1/tracks/{invalid_uuid}` | 404 TRACK_NOT_FOUND |
| 2 | `GET /api/v1/stream/{not_found_id}` | 404 TRACK_NOT_FOUND |
| 3 | `GET /api/v1/stream/{id}` con `Range: bytes=99999999-` | 416 RANGE_NOT_SATISFIABLE |
| 4 | Request sin token | 401 UNAUTHORIZED |
| 5 | Request con token sin permiso | 403 FORBIDDEN |

---

## Escenario C: Player ↔ Micro Server (E2E-07)

**Objetivo:** Verificar que Michi Music Player puede importar su biblioteca en Michi Micro Server y delegar reproducción.

### C.1 Pairing

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Player descubre Micro Server | service: `michi-micro-server` |
| 2 | Player inicia pairing | code + device_id |
| 3 | Usuario confirma en Micro Server | — |
| 4 | Player recibe token con permisos completos | permissions incluyen library.write, stream.read, etc. |

### C.2 Import Session

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | `POST /api/v1/import/session` con total_tracks + total_playlists | session_id + expires_at |
| 2 | `POST /api/v1/import/upload/{session_id}` con track_index, filename y chunk | accepted: true, track_id |
| 3 | Repetir upload para N tracks | Todos aceptados |
| 4 | `POST /api/v1/import/commit/{session_id}` | tracks_imported = N |
| 5 | Micro Server escanea y actualiza biblioteca | library/stats refleja nuevos tracks |

### C.3 Continue-on-Server

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Player envía estado actual a Micro Server | `POST /sync/state` |
| 2 | Micro Server puede continuar reproducción | playback/state refleja track + position |
| 3 | Mobile puede controlar Micro Server (hereda control) | play, pause, next |

---

## Escenario D: Micro Server ↔ Music Stream (E2E-09)

**Objetivo:** Verificar que Michi Micro Server puede parear, enviar sesión y controlar un Michi Music Stream físico.

**Server:** Micro Server (port 8500) + Stream Simulator — ver `tests/e2e_certification/SCENARIOS.md`.

### D.1 Discovery & Pairing

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Stream enciende y anuncia vía UDP (`224.0.0.167:53318`) | Micro Server recibe announce |
| 2 | Stream envía `POST /api/v1/pair/start` (device_type: receiver) a Micro Server | session_id + identidad del servidor |
| 3 | Usuario confirma el PIN en Micro Server | — |
| 4 | Stream confirma `POST /api/v1/pair/confirm` con session_id + pin y recibe token interno | device_id + token |

### D.2 Heartbeat

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Stream envía `POST /api/v1/receiver-lite/heartbeat` cada 10s | server_time, next_action |
| 2 | Micro Server ve Stream como online | GET /receivers muestra online: true |

### D.3 Session Start

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Micro Server envía `POST /api/v1/receiver-lite/session` con stream_url + token | session_id, state: playing |
| 2 | Stream reproduce audio desde stream_url | Audio audible en salida física |

### D.4 Volume Control

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Micro Server envía `POST /api/v1/receiver-lite/volume` con volume: 50 | volume: 50 |
| 2 | Volumen del Stream cambia | Audible en hardware |

### D.5 Session Stop

| Paso | Acción | Resultado esperado |
|------|--------|--------------------|
| 1 | Micro Server envía `DELETE /api/v1/receiver-lite/session` | previous_state: playing |
| 2 | Stream detiene reproducción | Silencio en salida física |

---

## Resumen de Escenarios

| Escenario | E2E ID (SCENARIOS.md) | Proyectos | Prioridad | Depende de |
|-----------|------------------------|-----------|-----------|------------|
| A | E2E-01 (pairing), E2E-03 (playback) | Mobile ↔ Player | Crítica | Player expone endpoints REST |
| B | E2E-04 (pairing), E2E-05 (download/sync) | Mobile ↔ Micro Server | Crítica | Micro Server funcional |
| C | E2E-07 | Player ↔ Micro Server | Alta | Ambos implementan import |
| D | E2E-09 | Micro Server ↔ Stream | Media | Firmware Stream funcional |

> NO existen escenarios E2E-02, E2E-06 ni E2E-10 en SCENARIOS.md.
