# Implementation Matrix — Michi Link API v1.0.0-alpha

## Implementation States

| Estado | Significado |
|--------|-------------|
| **stable** | Endpoint completamente implementado, probado y alineado con el contrato oficial. |
| **partial** | Endpoint implementado pero incompleto (faltan parámetros, respuestas parciales, wrapper legacy). |
| **stub** | Endpoint declarado pero devuelve 501/empty. |
| **legacy-wrapper** | Endpoint implementado con campos antiguos que requieren adaptación al contrato oficial. |
| **planned** | Endpoint planificado pero sin implementación. |
| **prototype** | Endpoint implementado como prototipo, sin validación en hardware real ni pruebas de producción. |
| **not-applicable** | El proyecto no implementa ni consume este endpoint (por su rol en el ecosistema). |
| **consume** | El proyecto consume el endpoint de otro servidor, no lo implementa. |

## E2E Status

| Valor | Significado |
|-------|-------------|
| **not-tested** | No probado end-to-end entre los proyectos involucrados. |
| **manual** | Probado manualmente por un desarrollador, sin test automatizado. |
| **automated** | Probado con test automatizado (CI/CD). |
| **certified** | Probado en múltiples entornos, con resultados documentados. |

---

| # | Endpoint | Método | Contrato Oficial | Player | Micro Server | Mobile | Music Stream | E2E Tested | Beta Blocker | Observaciones | Prioridad |
|---|----------|--------|------------------|--------|-------------|--------|-------------|------------|--------------|---------------|-----------|
| 1 | `/server/info` | GET | `service`, `name`, `server_id`, `version`, `api_version`, `michi_link_version`, `roles[]`, `features{}`, `auth{strategy}` | **stable** | **stable** | not-applicable | not-applicable | **manual** | no | Player service: `michi-music-player`, MS: `michi-micro-server`. Auth strategy difiere: PLAYER_PASSWORD vs SERVER_CODE. | Crítica |
| 2 | `/status` | GET | `{ status, version, uptime_seconds, timestamp }` | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | Endpoint público, sin auth | Alta |
| 3 | `/pair/start` | POST | `{ device_name, device_type }` → `{ pairing_code, device_id }` | **stable** | **stable** | **consume** | **prototype** (v1-lite) | **manual** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | | Crítica |
| 4 | `/pair/confirm` | POST | `{ device_id, pairing_code }` → `{ token, refresh_token, device_id, server_id }` | **stable** | **stable** | **consume** | **prototype** (v1-lite) | **manual** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | | Crítica |
| 5 | `/token/refresh` | POST | `{ refresh_token }` → `{ token, refresh_token }` | **not-applicable** | **stable** | **consume** | not-applicable | **not-tested** | **sí** — Mobile ↔ Micro | Player no implementa token_refresh. Mobile debe tolerar ausencia. | Alta |
| 6 | `/devices/revoke` | POST | `{ device_id }` → `{ success }` | **planned** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Media |
| 7 | `/library/stats` | GET | → `{ total_tracks, total_albums, total_artists, total_playlists }` | **stable** | **stable** | not-applicable | not-applicable | **manual** | no | | Alta |
| 8 | `/library/scan` | POST | → `{ success, scan_id }` | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Media |
| 9 | `/tracks` | GET | `?q, artist, album, genre, year, page, limit` → paginated | **stable** | **stable** | not-applicable | not-applicable | **manual** | no | | Alta |
| 10 | `/tracks/{id}` | GET | → Track completo | **stable** | **stable** | not-applicable | not-applicable | **manual** | no | | Alta |
| 11 | `/albums` | GET | `?q, artist, year, page, limit` → paginated | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Alta |
| 12 | `/albums/{id}` | GET | → Album + tracks | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Alta |
| 13 | `/artists` | GET | `?q, page, limit` → paginated | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Alta |
| 14 | `/artists/{id}` | GET | → Artist + albums | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Alta |
| 15 | `/search` | GET | `?q, type, limit` → { tracks, albums, artists, playlists } | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Alta |
| 16 | `/stream/{track_id}` | GET | `Range`, `Accept` → 206 / 200 | **stable** | **stable** | **consume** | **prototype** (v1-lite) | **manual** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | Player: Range tests existentes. MS: streaming nativo. | Crítica |
| 17 | `/download/{track_id}` | GET | `Accept` → 200 archivo completo | **planned** | **stable** | **consume** | not-applicable | **not-tested** | **sí** — Mobile ↔ Micro | Requiere permisos `download.read`. | Alta |
| 18 | `/artwork/{cover_id}` | GET | `?size` → imagen | **stable** | **stable** | **consume** | not-applicable | **manual** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | Cache headers | Alta |
| 19 | `/playlists` | GET | → paginated list | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Media |
| 20 | `/playlists` | POST | `{ name, description }` → 201 | **stable** | **planned** | not-applicable | not-applicable | **not-tested** | no | | Media |
| 21 | `/playlists/{id}` | GET | → Playlist | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Media |
| 22 | `/playlists/{id}` | PUT | `{ name, description }` → updated | **stable** | **planned** | not-applicable | not-applicable | **not-tested** | no | | Baja |
| 23 | `/playlists/{id}` | DELETE | → 204 | **stable** | **planned** | not-applicable | not-applicable | **not-tested** | no | | Baja |
| 24 | `/playlists/{id}/tracks` | GET | → lista de tracks | **stable** | **stable** | not-applicable | not-applicable | **not-tested** | no | | Media |
| 25 | `/playlists/{id}/tracks` | PUT | `{ track_ids }` → updated | **stable** | **planned** | not-applicable | not-applicable | **not-tested** | no | | Media |
| 26 | `/sync/manifest` | GET | → `{ cursor, generated_at, tracks{} }` | **stable** | **stable** | **consume** | not-applicable | **not-tested** | **sí** — Mobile ↔ Micro | cursor oficial | Crítica |
| 27 | `/sync/manifest/delta` | GET | `?device_id, cursor` → `{ cursor, added[], updated[], deleted[], playlists_updated[] }` | **stable** | **stable** | **consume** | not-applicable | **not-tested** | **sí** — Mobile ↔ Micro | cursor reemplaza since/manifest_id | Crítica |
| 28 | `/sync/state` | POST | `{ device_id, cursor, downloaded_tracks }` → `{ success }` | **stable** | **stable** | **consume** | not-applicable | **not-tested** | **sí** — Mobile ↔ Micro | | Alta |
| 29 | `/playback/state` | GET | → `{ state, track_id, position_ms, volume, shuffle, repeat }` | **stable** | **stable** | **consume** | not-applicable | **manual** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | `position_ms` oficial | Crítica |
| 30 | `/playback/control` | POST | `{ command, [position_ms\|volume] }` → `{ success, state }` | **stable** | **stable** | **consume** (envía) | not-applicable | **manual** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | `command` oficial, `action` legacy. Player y MS aceptan ambos. | Crítica |
| 31 | `/playback/session` | POST | `{ device_id, action, queue_id }` → `{ session_id }` | **stable** | **planned** | **planned** | not-applicable | **not-tested** | no | | Media |
| 32 | `/queue` | GET | → `{ id, current_index, items[], shuffle, repeat }` | **stable** | **stable** | **consume** | not-applicable | **not-tested** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | | Alta |
| 33 | `/queue/items` | POST | `{ track_ids, position }` → `{ items_added }` | **stable** | **stable** | **consume** (envía) | not-applicable | **not-tested** | **sí** — Mobile ↔ Player, Mobile ↔ Micro | | Alta |
| 34 | `/queue/jump` | POST | `{ queue_item_id }` → `{ current_index }` | **stable** | **stable** | **planned** | not-applicable | **not-tested** | no | | Media |
| 35 | `/queue/reorder` | PUT | `{ queue_item_ids }` → `{ success }` | **stable** | **planned** | **planned** | not-applicable | **not-tested** | no | | Baja |
| 36 | `/queue/items/{id}` | DELETE | → 204 | **stable** | **stable** | **planned** | not-applicable | **not-tested** | no | | Media |
| 37 | `/receivers` | GET | → paginated list | **planned** | **partial** | **planned** | not-applicable | **not-tested** | no | MS: lista de receivers conocidos vía heartbeat, sin integración real con Stream. | Baja |
| 38 | `/receivers/{id}` | GET | → Receiver detail | **planned** | **partial** | **planned** | not-applicable | **not-tested** | no | | Baja |
| 39 | `/receivers/{id}/session/start` | POST | `{ queue_id }` → `{ session_id }` | **planned** | **partial** | **planned** | not-applicable | **not-tested** | no | | Media |
| 40 | `/receivers/{id}/session/stop` | POST | → `{ success }` | **planned** | **partial** | **planned** | not-applicable | **not-tested** | no | | Media |
| 41 | `/receivers/{id}/volume` | POST | `{ volume }` → `{ volume }` | **planned** | **partial** | **planned** | not-applicable | **not-tested** | no | | Media |
| 42 | `/rooms` | GET | → paginated list | **planned** | **planned** | **planned** | not-applicable | **not-tested** | no | Multiroom v1 | Baja |
| 43 | `/rooms` | POST | `{ name, receiver_ids }` → 201 | **planned** | **planned** | **planned** | not-applicable | **not-tested** | no | | Baja |
| 44 | `/rooms/{id}` | GET | → Room detail | **planned** | **planned** | **planned** | not-applicable | **not-tested** | no | | Baja |
| 45 | `/rooms/{id}` | PUT | `{ name, receiver_ids }` → updated | **planned** | **planned** | **planned** | not-applicable | **not-tested** | no | | Baja |
| 46 | `/rooms/{id}` | DELETE | → 204 | **planned** | **planned** | **planned** | not-applicable | **not-tested** | no | | Baja |
| 47 | `/rooms/{id}/play` | POST | `{ queue_id }` → multiroom session | **planned** | **planned** | **planned** | not-applicable | **not-tested** | no | | Baja |
| 48 | `/events` | WS | WebSocket → eventos tiempo real | **stub** | **partial** | **planned** | not-applicable | **not-tested** | no | Player: stub (endpoint declarado, sin eventos reales). MS: broadcast básico de playback.state_changed. | Media |
| 49 | `/receiver/info` | GET | v1-lite → identidad | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | Sin validación en hardware real. | Alta |
| 50 | `/receiver/pair/start` | POST | v1-lite → pairing | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | | Alta |
| 51 | `/receiver/pair/confirm` | POST | v1-lite → confirm | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | | Alta |
| 52 | `/receiver/heartbeat` | POST | v1-lite cada 10s | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | | Alta |
| 53 | `/receiver/session/start` | POST | v1-lite → recibe URL stream | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | | Alta |
| 54 | `/receiver/session/stop` | POST | v1-lite → stop | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | | Alta |
| 55 | `/receiver/volume` | POST | v1-lite `{ volume }` | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | | Alta |
| 56 | `/receiver/firmware` | GET | v1-lite → version info | not-applicable | **partial** (consume) | not-applicable | **prototype** | **not-tested** | **sí** — MS ↔ Stream | | Media |

---

## Permisos Mínimos por Rol

| Rol | Permisos |
|-----|----------|
| `mobile_client` | `server.read`, `library.read`, `track.read`, `artwork.read`, `sync.read_manifest`, `sync.download_tracks`, `sync.download_covers`, `sync.upload_state`, `download.read`, `playback.read`, `playback.control`, `queue.read`, `queue.write` |
| `desktop_player` | `server.read`, `library.read`, `library.write`, `library.scan`, `track.read`, `track.write`, `artwork.read`, `playlist.read`, `playlist.write`, `stream.read`, `stream.transcode`, `sync.read_manifest`, `sync.upload_state`, `playback.read`, `playback.control`, `queue.read`, `queue.write`, `receiver.read`, `receiver.control`, `receiver.session`, `receiver.volume`, `room.read`, `room.write`, `system.read`, `system.write` |
| `library_server` | `server.read`, `library.read`, `library.write`, `track.read`, `track.write`, `artwork.read`, `playlist.read`, `playlist.write`, `sync.read_manifest`, `stream.read`, `stream.transcode`, `system.read` |
| `audio_receiver` | `server.read`, `stream.read`, `receiver.session`, `receiver.volume` |
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
| 13 | Micro Server ↔ Music Stream (v1-lite completo) | Music Stream |

---

## Versión

**v1.0.0-alpha** — Contrato oficial del ecosistema. Lista de beta blockers definida para coordinar la fase beta.
