# Implementation Matrix — Michi Link API v1.0.0-alpha

| # | Endpoint | Método | Contrato Oficial | Player | Micro Server | Mobile | Observaciones | Prioridad |
|---|----------|--------|------------------|--------|-------------|--------|---------------|-----------|
| 1 | `/server/info` | GET | `service`, `name`, `server_id`, `version`, `api_version`, `michi_link_version`, `roles[]`, `features{}`, `auth{}` | ✅ Implementado | ✅ Implementado | ❌ No aplica | Player usa `service: "michi-player"`, Micro Server usa `service: "michi-micro-server"` | Crítica |
| 2 | `/status` | GET | `{ status, version, uptime_seconds, timestamp }` | ❌ Pendiente | ❌ Pendiente | ❌ No aplica | Endpoint público, sin auth | Alta |
| 3 | `/pair/start` | POST | `{ device_name, device_type }` → `{ pairing_code, device_id }` | ✅ Implementado | ✅ Implementado | ✅ Implementado | | Crítica |
| 4 | `/pair/confirm` | POST | `{ device_id, pairing_code }` → `{ token, refresh_token, device_id, server_id }` | ✅ Implementado | ✅ Implementado | ✅ Implementado | | Crítica |
| 5 | `/token/refresh` | POST | `{ refresh_token }` → `{ token, refresh_token }` | ✅ Implementado | ✅ Implementado | ✅ Implementado | | Alta |
| 6 | `/devices/revoke` | POST | `{ device_id }` → `{ success }` | ❌ Pendiente | ✅ Implementado | ❌ No aplica | | Media |
| 7 | `/library/stats` | GET | → `{ total_tracks, total_albums, total_artists, total_playlists }` | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 8 | `/library/scan` | POST | → `{ success, scan_id }` | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Media |
| 9 | `/tracks` | GET | `?q, artist, album, genre, year, page, limit` → paginated | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 10 | `/tracks/{id}` | GET | → Track completo | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 11 | `/albums` | GET | `?q, artist, year, page, limit` → paginated | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 12 | `/albums/{id}` | GET | → Album + tracks | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 13 | `/artists` | GET | `?q, page, limit` → paginated | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 14 | `/artists/{id}` | GET | → Artist + albums | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 15 | `/search` | GET | `?q, type, limit` → { tracks, albums, artists, playlists } | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Alta |
| 16 | `/stream/{track_id}` | GET | `Range`, `Accept` → 206 Partial / 200 Full | ✅ Implementado | ✅ Implementado | ✅ Implementado | Formato nativo o transcoded según Accept | Crítica |
| 17 | `/download/{track_id}` | GET | `Accept` → 200 archivo completo | ❌ Pendiente | ✅ Implementado | ✅ Implementado | Requiere permisos `download.read` | Alta |
| 18 | `/artwork/{cover_id}` | GET | `?size` → imagen | ✅ Implementado | ✅ Implementado | ✅ Implementado | Cache headers | Alta |
| 19 | `/playlists` | GET | → paginated list | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Media |
| 20 | `/playlists` | POST | `{ name, description }` → 201 | ✅ Implementado | ❌ Pendiente | ❌ No aplica | | Media |
| 21 | `/playlists/{id}` | GET | → Playlist | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Media |
| 22 | `/playlists/{id}` | PUT | `{ name, description }` → updated | ✅ Implementado | ❌ Pendiente | ❌ No aplica | | Baja |
| 23 | `/playlists/{id}` | DELETE | → 204 | ✅ Implementado | ❌ Pendiente | ❌ No aplica | | Baja |
| 24 | `/playlists/{id}/tracks` | GET | → lista de tracks | ✅ Implementado | ✅ Implementado | ❌ No aplica | | Media |
| 25 | `/playlists/{id}/tracks` | PUT | `{ track_ids }` → updated | ✅ Implementado | ❌ Pendiente | ❌ No aplica | | Media |
| 26 | `/sync/manifest` | GET | → `{ cursor, generated_at, tracks{}, albums{}, artists{}, playlists{} }` | ✅ Implementado | ✅ Implementado | ✅ Implementado (lectura) | cursor es el campo oficial | Crítica |
| 27 | `/sync/manifest/delta` | GET | `?device_id, cursor` → `{ cursor, added[], updated[], deleted[], playlists_updated[] }` | ✅ Implementado | ✅ Implementado | ✅ Implementado (lectura) | cursor reemplaza since/manifest_id | Crítica |
| 28 | `/sync/state` | POST | `{ device_id, cursor, downloaded_tracks }` → `{ success }` | ✅ Implementado | ✅ Implementado | ✅ Implementado | | Alta |
| 29 | `/playback/state` | GET | → `{ state, current_track, position_ms, volume, shuffle, repeat }` | ✅ Implementado | ✅ Implementado | ❌ Solo lectura remota | `position_ms` oficial | Crítica |
| 30 | `/playback/control` | POST | `{ command, [position_ms|volume] }` → `{ success, state }` | ✅ Implementado | ✅ Implementado | ✅ Implementado (envía) | `command` oficial, `action` legacy | Crítica |
| 31 | `/playback/session` | POST | `{ device_id, action, queue_id }` → `{ session_id }` | ✅ Implementado | ❌ Pendiente | ❌ Pendiente | | Media |
| 32 | `/queue` | GET | → `{ id, current_index, items[], shuffle, repeat }` | ✅ Implementado | ✅ Implementado | ❌ Solo lectura remota | `duration_ms` en items | Alta |
| 33 | `/queue/items` | POST | `{ track_ids, position }` → `{ items_added }` | ✅ Implementado | ✅ Implementado | ✅ Implementado (envía) | | Alta |
| 34 | `/queue/jump` | POST | `{ queue_item_id }` → `{ current_index }` | ✅ Implementado | ✅ Implementado | ❌ Pendiente | | Media |
| 35 | `/queue/reorder` | PUT | `{ queue_item_ids }` → `{ success }` | ✅ Implementado | ❌ Pendiente | ❌ Pendiente | | Baja |
| 36 | `/queue/items/{id}` | DELETE | → 204 | ✅ Implementado | ✅ Implementado | ❌ Pendiente | | Media |
| 37 | `/receivers` | GET | → paginated list | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | Consumido desde Player o Mobile | Baja |
| 38 | `/receivers/{id}` | GET | → Receiver detail | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Baja |
| 39 | `/receivers/{id}/session/start` | POST | `{ queue_id }` → `{ session_id }` | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Media |
| 40 | `/receivers/{id}/session/stop` | POST | → `{ success }` | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Media |
| 41 | `/receivers/{id}/volume` | POST | `{ volume }` → `{ volume }` | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Media |
| 42 | `/rooms` | GET | → paginated list | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | Multiroom v1 | Baja |
| 43 | `/rooms` | POST | `{ name, receiver_ids }` → 201 | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Baja |
| 44 | `/rooms/{id}` | GET | → Room detail | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Baja |
| 45 | `/rooms/{id}` | PUT | `{ name, receiver_ids }` → updated | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Baja |
| 46 | `/rooms/{id}` | DELETE | → 204 | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Baja |
| 47 | `/rooms/{id}/play` | POST | `{ queue_id }` → multiroom session | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | | Baja |
| 48 | `/events` | WS | WebSocket → eventos en tiempo real | ❌ Pendiente | ❌ Pendiente | ❌ Pendiente | No es transporte principal | Media |
| 49 | `/receiver/info` | GET | v1-lite → identidad | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | Implementado en firmware | Alta |
| 50 | `/receiver/pair/start` | POST | v1-lite → pairing | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | | Alta |
| 51 | `/receiver/pair/confirm` | POST | v1-lite → confirm | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | | Alta |
| 52 | `/receiver/heartbeat` | POST | v1-lite cada 10s | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | | Alta |
| 53 | `/receiver/session/start` | POST | v1-lite → recibe URL stream | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | | Alta |
| 54 | `/receiver/session/stop` | POST | v1-lite → stop | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | | Alta |
| 55 | `/receiver/volume` | POST | v1-lite `{ volume }` | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | | Alta |
| 56 | `/receiver/firmware` | GET | v1-lite → version info | ❌ No aplica | ❌ Pendiente (consume) | ❌ No aplica | | Media |

## Permisos Mínimos por Rol

| Rol | Permisos |
|-----|----------|
| `mobile_client` | `server.read`, `library.read`, `track.read`, `artwork.read`, `sync.read_manifest`, `sync.download_tracks`, `sync.download_covers`, `sync.upload_state`, `download.read`, `playback.read`, `playback.control`, `queue.read`, `queue.write` |
| `desktop_player` | `server.read`, `library.read`, `library.write`, `library.scan`, `track.read`, `track.write`, `artwork.read`, `playlist.read`, `playlist.write`, `stream.read`, `stream.transcode`, `sync.read_manifest`, `sync.upload_state`, `playback.read`, `playback.control`, `queue.read`, `queue.write`, `receiver.read`, `receiver.control`, `receiver.session`, `receiver.volume`, `room.read`, `room.write`, `system.read`, `system.write` |
| `library_server` | `server.read`, `library.read`, `library.write`, `track.read`, `track.write`, `artwork.read`, `playlist.read`, `playlist.write`, `sync.read_manifest`, `stream.read`, `stream.transcode`, `system.read` |
| `audio_receiver` | `server.read`, `stream.read`, `receiver.session`, `receiver.volume` |
| `remote_controller` | `server.read`, `library.read`, `track.read`, `artwork.read`, `playlist.read`, `playlist.write`, `playback.read`, `playback.control`, `queue.read`, `queue.write`, `receiver.read`, `receiver.control`, `receiver.volume`, `room.read` |

## Versión

**v1.0.0-alpha** — Pendiente de congelar como contrato oficial.
