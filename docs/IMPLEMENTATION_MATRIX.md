# Implementation Matrix — Michi Link API v1.0.0-alpha.1

## Evidence Levels

| Nivel | Significado |
|-------|-------------|
| **CONTRACT_PASS** | Pasa la validación de contrato del repo: schemas, OpenAPI y ejemplos (248 checks) |
| **RUST_REFERENCE_PASS** | Pasa la suite del crate de referencia `crates/michi-identity` (88 tests, clippy 0 warnings) |
| **CROSS_LAYER_PASS** | Pasa los checks cruzados schemas ↔ OpenAPI ↔ crate (84 checks) |
| **BUNDLE_PASS** | El bundle `contracts/receiver-v1-lite/` es reproducible byte a byte (job CI `bundle-reproducibility`) |
| **NOT_TESTED** | Sin evidencia de certificación |
| **UNIT_PASS** | Pasa test unitario aislado |
| **MOCK_PASS** | Pasa contra servidor mock |
| **LOCAL_E2E_PASS** | Pasa contra localhost real |
| **NETWORK_E2E_PASS** | Pasa en LAN real |
| **DEVICE_E2E_PASS** | Pasa en hardware físico real |
| **FAIL** | No pasa el test |
| **consume** | El proyecto consume el endpoint de otro servidor, no lo implementa |
| **not-applicable** | El proyecto no implementa ni consume este endpoint (por su rol en el ecosistema) |

Evidencia de contrato verificada (2026-08-13): `tests/contract` 248 checks PASS (CONTRACT_PASS), `tests/identity_contract` 22 checks PASS (CONTRACT_PASS), `tests/cross_layer` 84 checks PASS (CROSS_LAYER_PASS), `crates/michi-identity` 88 tests + clippy 0 warnings (RUST_REFERENCE_PASS), redocly 0 errors, `scripts/contract-policy.py` PASS, y el job `bundle-reproducibility` regenera `contracts/receiver-v1-lite/` sin diff (BUNDLE_PASS). El set canónico de errores tiene **20 códigos** (ver `schemas/error.schema.json`).

Fuente de evidencia por endpoint: `docs/BETA_READINESS_CHECKLIST.md`. Los niveles `stable`/`DONE`/`manual` NO son niveles de evidencia y están prohibidos en esta matriz.

## E2E Status

| Valor | Significado |
|-------|-------------|
| **NOT_TESTED** | No probado end-to-end entre los proyectos involucrados. |
| **NETWORK_E2E_PASS** | Certificado en LAN real (reporte en `tests/e2e_certification/reports/`). |
| **DEVICE_E2E_PASS** | Certificado en hardware físico real. |

> `tests/e2e_certification/reports/` está vacío: todos los flujos E2E están **NOT_TESTED**.

---

| # | Endpoint | Método | Contrato Oficial | Player | Micro Server | Mobile | Music Stream | E2E Status | Beta Blocker | Observaciones | Prioridad |
|---|----------|--------|------------------|--------|-------------|--------|-------------|------------|--------------|---------------|-----------|
| 1 | `/server/info` | GET | `service`, `name`, `server_id`, `version`, `api_version`, `roles[]`, `features{}`, `auth{strategy}` | NOT_TESTED | **UNIT_PASS** | not-applicable | NOT_TESTED | NOT_TESTED | no | Player service: `michi-music-player`, MS: `michi-micro-server`. Auth strategy difiere: PLAYER_PASSWORD vs SERVER_CODE. Stream: perfil receiver v1-lite exacto (`michi-stream-*`, `roles: ["audio_receiver"]`, `RECEIVER_BUTTON`). | Crítica |
| 2 | `/status` | GET | `{ status, version, uptime_seconds, timestamp }` | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | Endpoint público, sin auth | Alta |
| 3 | `/pair/start` | POST | `{ device_name, device_type, roles[], auth_strategy, michi_id, public_key, challenge_nonce, challenge_signature }` → `{ session_id, expires_at, attempts_remaining, server_michi_id, server_public_key }` | NOT_TESTED | **UNIT_PASS** | **consume** | NOT_TESTED | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | El PIN se muestra en el servidor y nunca viaja por la red | Crítica |
| 4 | `/pair/confirm` | POST | `{ session_id, pin, michi_id, public_key }` → `{ token, refresh_token?, expires_in, device_id, server_id }` | NOT_TESTED | **UNIT_PASS** | **consume** | NOT_TESTED | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | | Crítica |
| 5 | `/token/refresh` | POST | `{ refresh_token }` → `{ token, refresh_token }` | not-applicable | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Micro | Player no implementa token_refresh. Mobile debe tolerar ausencia. | Alta |
| 6 | `/pair/status` | GET | `?session_id` → `{ session_id, status, expires_at, attempts_remaining }` (`pending`/`confirmed`/`expired`/`locked`) | NOT_TESTED | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | no | | Media |
| 7 | `/library/stats` | GET | → `{ total_tracks, total_albums, total_artists, total_playlists }` | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 8 | `/library/scan` | POST | → `{ success, scan_id }` | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Media |
| 9 | `/tracks` | GET | `?q, artist, album, genre, year, page, limit` → paginated | NOT_TESTED | **UNIT_PASS** | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 10 | `/tracks/{id}` | GET | → Track completo | NOT_TESTED | **UNIT_PASS** | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 11 | `/albums` | GET | `?q, artist, year, page, limit` → paginated | NOT_TESTED | **UNIT_PASS** | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 12 | `/albums/{id}` | GET | → Album + tracks | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 13 | `/artists` | GET | `?q, page, limit` → paginated | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 14 | `/artists/{id}` | GET | → Artist + albums | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 15 | `/search` | GET | `?q, type, limit` → { tracks, albums, artists, playlists } | NOT_TESTED | **UNIT_PASS** | not-applicable | not-applicable | NOT_TESTED | no | | Alta |
| 16 | `/stream/{track_id}` | GET | `Range`, `Accept` → 206 / 200 | NOT_TESTED | **UNIT_PASS** | **consume** | NOT_TESTED | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | Player: Range tests existentes. MS: streaming nativo. | Crítica |
| 17 | `/download/{track_id}` | GET | `Accept` → 200 archivo completo | NOT_TESTED | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Micro | Requiere permisos `download.read`. | Alta |
| 18 | `/artwork/{cover_id}` | GET | `?size` → imagen | NOT_TESTED | NOT_TESTED | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | Cache headers | Alta |
| 19 | `/playlists` | GET | → paginated list | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Media |
| 20 | `/playlists` | POST | `{ name, description }` → 201 | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Media |
| 21 | `/playlists/{id}` | GET | → Playlist | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Media |
| 22 | `/playlists/{id}` | PUT | `{ name, description }` → updated | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Baja |
| 23 | `/playlists/{id}` | DELETE | → 204 | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Baja |
| 24 | `/playlists/{id}/tracks` | GET | → lista de tracks | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Media |
| 25 | `/playlists/{id}/tracks` | PUT | `{ track_ids }` → updated | NOT_TESTED | NOT_TESTED | not-applicable | not-applicable | NOT_TESTED | no | | Media |
| 26 | `/sync/manifest` | GET | → `{ cursor, generated_at, tracks{} }` | NOT_TESTED | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Micro | cursor oficial | Crítica |
| 27 | `/sync/manifest/delta` | GET | `?device_id, cursor` → `{ cursor, added[], updated[], deleted[], playlists_updated[] }` | NOT_TESTED | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Micro | cursor reemplaza since/manifest_id | Crítica |
| 28 | `/sync/state` | POST | `{ device_id, cursor, downloaded_tracks }` → `{ success }` | NOT_TESTED | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Micro | | Alta |
| 29 | `/playback/state` | GET | → `{ state, track_id, position_ms, volume, shuffle, repeat }` | NOT_TESTED | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | `position_ms` oficial | Crítica |
| 30 | `/playback/control` | POST | `{ command, [position_ms\|volume] }` → `{ success, state }` | NOT_TESTED | **UNIT_PASS** | **consume** (envía) | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | `command` oficial, `action` legacy. Player y MS aceptan ambos. | Crítica |
| 31 | `/playback/session` | POST | `{ device_id, action, queue_id }` → `{ session_id }` | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Media |
| 32 | `/queue` | GET | → `{ id, current_index, items[], shuffle, repeat }` | NOT_TESTED | **UNIT_PASS** | **consume** | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | | Alta |
| 33 | `/queue/items` | POST | `{ track_ids, position }` → `{ items_added }` | NOT_TESTED | **UNIT_PASS** | **consume** (envía) | not-applicable | NOT_TESTED | **sí** — Mobile ↔ Player, Mobile ↔ Micro | | Alta |
| 34 | `/queue/jump` | POST | `{ queue_item_id }` → `{ current_index }` | NOT_TESTED | **UNIT_PASS** | NOT_TESTED | not-applicable | NOT_TESTED | no | | Media |
| 35 | `/queue/reorder` | PUT | `{ queue_item_ids }` → `{ success }` | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Baja |
| 36 | `/queue/items/{id}` | DELETE | → 204 | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Media |
| 37 | `/receivers` | GET | → `{ receivers: [DeviceInfo] }` | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | MS: lista de receptores descubiertos/registrados vía `GET /receivers` y `POST /receivers/discover`; sin integración real con Stream. | Baja |
| 38 | `/receivers/{id}` | GET | → Receiver detail | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Baja |
| 39 | `/receivers/discover` | POST | `?timeout` → `{ discovered: [DeviceInfo] }` | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | Discovery de receptores en la red (mDNS/UDP firmado). | Media |
| 40 | `/receivers/{id}/session/start` | POST | **Retirado** — sin ruta activa en OpenAPI | not-applicable | not-applicable | not-applicable | not-applicable | not-applicable | no | Sustituido por `POST /receiver-lite/session` en el receptor. | — |
| 41 | `/receivers/{id}/session/stop` + `/receivers/{id}/volume` | POST | **Retirados** — sin rutas activas en OpenAPI | not-applicable | not-applicable | not-applicable | not-applicable | not-applicable | no | Sustituidos por `PATCH/DELETE /receiver-lite/session`. | — |
| 42 | `/rooms` | GET | → paginated list | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | Multiroom v1 | Baja |
| 43 | `/rooms` | POST | `{ name, receiver_ids }` → 201 | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Baja |
| 44 | `/rooms/{id}` | GET | → Room detail | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Baja |
| 45 | `/rooms/{id}` | PUT | `{ name, receiver_ids }` → updated | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Baja |
| 46 | `/rooms/{id}` | DELETE | → 204 | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Baja |
| 47 | `/rooms/{id}/play` | POST | `{ queue_id }` → multiroom session | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | | Baja |
| 48 | `/events` | WS | WebSocket → eventos tiempo real | NOT_TESTED | NOT_TESTED | NOT_TESTED | not-applicable | NOT_TESTED | no | Player: stub (endpoint declarado, sin eventos reales). MS: broadcast básico de playback.state_changed. | Media |
| 49 | `/receiver-lite/session` | POST | v1-lite → `{ transport, codec, sample_rate, bit_depth, channels, packet_ms, buffer_ms, payload_type, ssrc, volume }` → `201 { session_id, session_token, lease_seconds: 30, effective{…, stream_port} }` | not-applicable | **consume** | not-applicable | NOT_TESTED | NOT_TESTED | **sí** — Micro ↔ Stream Simulator | MS crea la sesión RTP/UDP PCM (48 kHz/16-bit/2ch/10 ms/PT 97); el receptor elige puerto UDP 49152..65535 y fija la IP RTP a la IP TCP del request. | Alta |
| 50 | `/pair/start` + `/pair/status` + `/pair/confirm` (receiver) | POST/GET | v1-lite → pairing `RECEIVER_BUTTON` con ventana física de 120 s; token emitido por el receptor (`expires_in: 0`), PIN solo local | not-applicable | **consume** | not-applicable | NOT_TESTED | NOT_TESTED | **sí** — Micro ↔ Stream Simulator | El pairing de receivers usa los endpoints de pairing canónicos; `device_type: "server"`, `roles: ["music_server"]`. | Alta |
| 51 | `/receiver-lite/session` | GET | v1-lite → estado `starting/playing/paused/stopping` + métricas de paquetes | not-applicable | **consume** | not-applicable | NOT_TESTED | NOT_TESTED | **sí** — Micro ↔ Stream Simulator | Nunca devuelve `session_token`. | Alta |
| 52 | `/receiver-lite/session` | PATCH | v1-lite → `{ volume, paused }` | not-applicable | **consume** | not-applicable | NOT_TESTED | NOT_TESTED | **sí** — Micro ↔ Stream Simulator | Sustituye al endpoint `/volume` retirado. | Alta |
| 53 | `/receiver-lite/heartbeat` | POST | v1-lite cada 10s → `{ session_id, sequence, sent_at_ms }` → renueva lease de 30 s | not-applicable | **consume** | not-applicable | NOT_TESTED | NOT_TESTED | **sí** — Micro ↔ Stream Simulator | `sequence` estrictamente creciente; replay → 409. | Alta |
| 54 | `/receiver-lite/session` | DELETE | v1-lite → cierre seguro, `204` | not-applicable | **consume** | not-applicable | NOT_TESTED | NOT_TESTED | **sí** — Micro ↔ Stream Simulator | Libera RTP/socket/buffers y borra token en RAM. | Alta |
| 55 | `/receiver-lite/{now-playing, diagnostics, firmware}` | PUT/GET/POST | v1-lite → extensiones opcionales por feature flags (`now_playing`, `diagnostics`, `ota`) | not-applicable | **consume** | not-applicable | NOT_TESTED | NOT_TESTED | **sí** — Micro ↔ Stream Simulator | Shapes no congelados hasta certificación; OTA exige permiso `receiver.ota` (no otorgado por defecto). | Media |

---

## Permisos Mínimos por Rol

| Rol | Permisos |
|-----|----------|
| `mobile_player` | `server.read`, `library.read`, `track.read`, `artwork.read`, `sync.read_manifest`, `sync.download_tracks`, `sync.download_covers`, `sync.upload_state`, `download.read`, `playback.read`, `playback.control`, `queue.read`, `queue.write` |
| `desktop_player` | `server.read`, `library.read`, `library.write`, `library.scan`, `track.read`, `track.write`, `artwork.read`, `playlist.read`, `playlist.write`, `stream.read`, `stream.transcode`, `sync.read_manifest`, `sync.upload_state`, `playback.read`, `playback.control`, `queue.read`, `queue.write`, `receiver.read`, `receiver.control`, `receiver.session`, `receiver.volume`, `room.read`, `room.write`, `system.read`, `system.write` |
| `music_server` | `server.read`, `library.read`, `library.write`, `track.read`, `track.write`, `artwork.read`, `playlist.read`, `playlist.write`, `sync.read_manifest`, `stream.read`, `stream.transcode`, `system.read` |
| `audio_receiver` | `receiver.status`, `receiver.session`, `receiver.volume`, `receiver.now_playing` |
| `remote_controller` | `server.read`, `library.read`, `track.read`, `artwork.read`, `playlist.read`, `playlist.write`, `playback.read`, `playback.control`, `queue.read`, `queue.write`, `receiver.read`, `receiver.control`, `receiver.volume`, `room.read` |

---

## Resumen de Beta Blockers

| # | Flujo | Bloquea beta de |
|---|-------|----------------|
| 1 | Mobile ↔ Player pairing | Mobile app |
| 2 | Mobile ↔ Player stream/artwork | Mobile app |
| 3 | Mobile ↔ Player playback/control | Mobile app |
| 4 | Mobile ↔ Micro Server pairing | Mobile app |
| 5 | Mobile ↔ Micro Server token refresh | Mobile app |
| 6 | Mobile ↔ Micro Server download/stream | Mobile app |
| 7 | Mobile ↔ Micro Server playback/control | Mobile app |
| 8 | Mobile ↔ Micro Server sync manifest/delta/state | Mobile app |
| 9 | Player ↔ Micro Server import/session | Micro Server |
| 10 | Player ↔ Micro Server import/upload | Micro Server |
| 11 | Player ↔ Micro Server import/commit | Micro Server |
| 12 | Micro Server autonomous playback | Micro Server |
| 13 | Micro ↔ Stream Simulator (receiver v1-lite completo) | Music Stream |

---

## Versión

**v1.0.0-alpha.1** — Contrato oficial del ecosistema, con el perfil receiver v1-lite congelado (ADR-0001) y publicado como bundle versionado `contracts/receiver-v1-lite/` (`1.0.0-alpha.1`). Lista de beta blockers definida para coordinar la fase beta. La matriz usa niveles de evidencia reales; sin reportes E2E (`tests/e2e_certification/reports/` vacío) todo flujo E2E — incluido Micro ↔ Stream Simulator — es NOT_TESTED, y ningún hardware está certificado.
