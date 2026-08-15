# Beta Readiness Checklist — Michi Link API v1.0.0-alpha.1

## Instrucciones

Cada ítem debe estar en estado **PASS** antes de declarar la beta. Si algún ítem está **FAIL** o **NOT_TESTED**, la beta no puede comenzar.

### Columnas

| Columna | Significado |
|---------|-------------|
| Certification Level | `NOT_TESTED` / `UNIT_PASS` / `MOCK_PASS` / `LOCAL_E2E_PASS` / `NETWORK_E2E_PASS` / `DEVICE_E2E_PASS` / `FAIL` |
| Evidence | Enlace a test, reporte E2E o commit que demuestra el pase |
| Test command | Comando exacto para reproducir la verificación |
| Last verified | Fecha ISO 8601 de la última verificación |
| Blocking | `sí` si es beta blocker, `no` si no |

### Niveles de certificación

| Nivel | Código | Significado |
|-------|--------|-------------|
| No probado | NOT_TESTED | No ejecutado |
| Unitario | UNIT_PASS | Pasa test unitario aislado |
| Mock | MOCK_PASS | Pasa contra servidor mock |
| Local E2E | LOCAL_E2E_PASS | Pasa contra localhost real |
| Red E2E | NETWORK_E2E_PASS | Pasa en LAN real |
| Hardware E2E | DEVICE_E2E_PASS | Pasa en hardware físico real |
| Falla | FAIL | No pasa el test |

---

## 1. Contrato

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 1.1 | server-info.schema.json service enum tiene `michi-music-player` | UNIT_PASS | schemas/server-info.schema.json | `npx ajv validate -s schemas/server-info.schema.json -d examples/server-info-player.json` | 2026-07-15 | no |
| 1.2 | server-info.schema.json features son booleanos | UNIT_PASS | schemas/server-info.schema.json | test contract negativo: features con objetos falla | 2026-07-15 | no |
| 1.3 | server-info.schema.json auth.required es obligatorio | UNIT_PASS | schemas/server-info.schema.json | test contract negativo: falta auth.required | 2026-07-15 | no |
| 1.4 | server-info.schema.json api_version validada estrictamente (enum v1/v1-lite; `additionalProperties: false` rechaza campos extra, incl. `michi_link_version` retirado) | UNIT_PASS | tests/contract/validate.js | test contract negativo: `api_version: "1.0.0"` rechazado; `michi_link_version` extra rechazado (248 checks) | 2026-08-13 | no | <!-- michi-policy:exclude -->
| 1.5 | playback-control.schema.json requiere command | UNIT_PASS | schemas/playback-control.schema.json | test contract: command requerido, action rechazado | 2026-07-15 | no |
| 1.6 | sync-delta.schema.json usa cursor (string) | UNIT_PASS | schemas/sync-delta.schema.json | npm test valida sync-delta.json | 2026-07-15 | no |
| 1.7 | error.schema.json usa { error: { code, message, details } } | UNIT_PASS | schemas/error.schema.json | npm test valida error examples | 2026-07-15 | no |
| 1.8 | Ejemplos validan contra schemas (tests/contract: 248 checks) | UNIT_PASS | tests/contract/validate.js | `cd tests/contract && npm test` | 2026-08-13 | no |
| 1.9 | Wire base64url estricto (michi_id/public_key 43 chars, signature 86, sin padding ni `+`/`/`/`=`) | UNIT_PASS | tests/identity_contract/validate.js | `cd tests/identity_contract && npm test` (22 checks) | 2026-08-13 | no |
| 1.10 | OpenAPI spec actualizada | UNIT_PASS | openapi/michi-link-v1.yaml | — | 2026-07-15 | no |
| 1.11 | Bundle receiver v1-lite reproducible (VERSION 1.0.0-alpha.1 + manifest SHA-256, regen sin diff) | BUNDLE_PASS | contracts/receiver-v1-lite/ + .github/workflows/contract.yml (job bundle-reproducibility) | `python3 scripts/build-receiver-bundle.py && git diff --exit-code -- contracts/receiver-v1-lite` | 2026-08-13 | no |

## 2. Autenticación

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 2.1 | Player expone /server/info con auth.strategy=PLAYER_PASSWORD | NOT_TESTED | — | `curl http://PLAYER:8400/api/v1/server/info` | — | **sí** |
| 2.2 | Micro Server expone /server/info con auth.strategy=SERVER_CODE | UNIT_PASS | crates/michi-api/src/routes/v1/server.rs | test_v1_server_info en michi-api | 2026-07-15 | **sí** |
| 2.3 | Player token_refresh=false | NOT_TESTED | — | GET /server/info → auth.token_refresh | — | **sí** |
| 2.4 | Micro Server token_refresh=true | UNIT_PASS | crates/michi-api/src/routes/v1/pair.rs | test_v1_token_refresh | 2026-07-15 | **sí** |
| 2.5 | Mobile detecta estrategia y actúa en consecuencia | NOT_TESTED | — | Mobile conecta a Player y Micro | — | **sí** |
| 2.6 | Receiver expone auth.strategy=RECEIVER_BUTTON | NOT_TESTED | — | `GET /server/info` en Stream (`api_version: "v1-lite"`, `auth.strategy`) | — | no |

## 3. Pairing

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 3.1 | Player acepta POST /pair/start | NOT_TESTED | — | `curl -X POST http://PLAYER:8400/api/v1/pair/start -H 'Content-Type: application/json' -d '{"device_name":"test","device_type":"mobile","roles":["mobile_player"],"auth_strategy":"ED25519_CHALLENGE","michi_id":"<43 chars base64url>","public_key":"<43 chars base64url>","challenge_nonce":"<>=22 chars base64url>","challenge_signature":"<86 chars base64url>"}'` | — | **sí** |
| 3.2 | Player acepta POST /pair/confirm | NOT_TESTED | — | `curl -X POST http://PLAYER:8400/api/v1/pair/confirm -H 'Content-Type: application/json' -d '{"session_id":"<uuid>","pin":"482391","michi_id":"<43 chars base64url>","public_key":"<43 chars base64url>"}'` | — | **sí** |
| 3.3 | Micro Server acepta POST /pair/start | UNIT_PASS | crates/michi-api/src/routes/v1/pair.rs | test_v1_pair_start | 2026-07-15 | **sí** |
| 3.4 | Micro Server acepta POST /pair/confirm | UNIT_PASS | crates/michi-api/src/routes/v1/pair.rs | test_v1_pair_confirm | 2026-07-15 | **sí** |
| 3.5 | Mobile puede pairar con Player | NOT_TESTED | — | Escenario E2E-01 | — | **sí** |
| 3.6 | Mobile puede pairar con Micro Server | NOT_TESTED | — | Escenario E2E-04 | — | **sí** |
| 3.7 | La sesión de pairing expira (5 min) | RUST_REFERENCE_PASS | crates/michi-identity/src/pairing.rs | MAX_SESSION_DURATION + test_expired_session (88 tests) | 2026-08-05 | no |
| 3.8 | Pairing confirm devuelve token + permisos canónicos | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_pair_confirm_returns_canonical_permissions | 2026-07-15 | no |
| 3.9 | Límites del registry de pairing (1024 global / 8 por origen / 4 por identidad / 20 starts por min) | RUST_REFERENCE_PASS | crates/michi-identity/src/pairing.rs | constantes MAX_* + test_rate_limit_pair_start (88 tests) | 2026-08-05 | no |

## 4. Biblioteca

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 4.1 | Player expone GET /tracks | NOT_TESTED | — | `curl http://PLAYER:8400/api/v1/tracks?limit=5` | — | no |
| 4.2 | Micro Server expone GET /tracks | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_tracks | 2026-07-15 | no |
| 4.3 | Player expone GET /albums | NOT_TESTED | — | — | — | no |
| 4.4 | Micro Server expone GET /albums | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_albums | 2026-07-15 | no |
| 4.5 | Player expone GET /search | NOT_TESTED | — | — | — | no |
| 4.6 | Micro Server expone GET /search | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_search | 2026-07-15 | no |
| 4.7 | Paginación funciona (page, limit, total, has_more) | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_tracks_pagination | 2026-07-15 | no |
| 4.8 | Filtros por artista, álbum, género, año | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_tracks_filter | 2026-07-15 | no |

## 5. Streaming

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 5.1 | Player sirve GET /stream/{id} con Range | NOT_TESTED | — | `curl -H 'Range: bytes=0-1023' http://PLAYER:8400/api/v1/stream/{id}` | — | **sí** |
| 5.2 | Micro Server sirve GET /stream/{id} con Range | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_stream_range_request | 2026-07-15 | **sí** |
| 5.3 | Responde 206 Partial Content con Range válido | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_stream_range_request | 2026-07-15 | **sí** |
| 5.4 | Responde 416 RANGE_NOT_SATISFIABLE con Range inválido | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_stream_range_not_satisfiable | 2026-07-15 | **sí** |
| 5.5 | Responde 404 TRACK_NOT_FOUND si track no existe | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_stream_track_not_found | 2026-07-15 | no |
| 5.6 | Responde 200 OK sin Range | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_stream | 2026-07-15 | no |
| 5.7 | Transcoding on-the-fly (mp3, ogg, hls) | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_hls_format_recognized | 2026-07-15 | no |

## 6. Descargas

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 6.1 | Micro Server sirve GET /download/{id} | UNIT_PASS | crates/michi-api/src/routes/v1/stream.rs | test_v1_stream_download | 2026-07-15 | **sí** |
| 6.2 | Requiere permiso download.read | UNIT_PASS | crates/michi-api/tests/api.rs (repo michi-micro-server) | test_v1_pair_confirm_returns_canonical_permissions | 2026-07-15 | **sí** |
| 6.3 | Responde Content-Disposition: attachment | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_stream_download_attachment | 2026-07-15 | no |
| 6.4 | Player implementa GET /download | NOT_TESTED | — | — | — | no |
| 6.5 | Mobile puede descargar desde Micro Server | NOT_TESTED | — | Escenario E2E-05 | — | **sí** |

## 7. Sincronización

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 7.1 | Micro Server expone GET /sync/manifest | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_sync_manifest | 2026-07-15 | **sí** |
| 7.2 | Micro Server expone GET /sync/manifest/delta?cursor= | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_sync_manifest_delta_with_cursor | 2026-07-15 | **sí** |
| 7.3 | Micro Server acepta POST /sync/state | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_sync_state | 2026-07-15 | **sí** |
| 7.4 | Manifest incluye cursor | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_sync_manifest_has_cursor | 2026-07-15 | no |
| 7.5 | Delta acepta cursor, since y manifest_id (legacy) | UNIT_PASS | crates/michi-api/src/routes/v1/sync.rs | test_v1_sync_manifest_delta_with_cursor | 2026-07-15 | no |
| 7.6 | Player expone sync endpoints | NOT_TESTED | — | — | — | no |
| 7.7 | Mobile puede sincronizar desde Micro Server | NOT_TESTED | — | Escenario E2E-05 | — | **sí** |

## 8. Reproducción

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 8.1 | Player expone GET /playback/state | NOT_TESTED | — | `curl http://PLAYER:8400/api/v1/playback/state` | — | **sí** |
| 8.2 | Player acepta POST /playback/control con command | NOT_TESTED | — | `curl -X POST http://PLAYER:8400/api/v1/playback/control -d '{"command":"play"}'` | — | **sí** |
| 8.3 | Micro Server expone GET /playback/state | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_playback_state_returns_state_field | 2026-07-15 | **sí** |
| 8.4 | Micro Server acepta POST /playback/control con command | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_playback_control | 2026-07-15 | **sí** |
| 8.5 | Micro Server acepta action como legacy | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_playback_control_legacy_action | 2026-07-15 | no |
| 8.6 | seek usa position_ms | UNIT_PASS | crates/michi-api/src/routes/v1/playback.rs | test_v1_playback_control_seek | 2026-07-15 | no |
| 8.7 | set_volume usa volume 0-100 | UNIT_PASS | crates/michi-api/src/routes/v1/playback.rs | test_v1_playback_control_volume | 2026-07-15 | no |
| 8.8 | Mobile puede controlar Player | NOT_TESTED | — | Escenario E2E-03 | — | **sí** |
| 8.9 | Mobile puede controlar Micro Server | NOT_TESTED | — | Escenario E2E-06 (no dedicated scenario in SCENARIOS.md) | — | **sí** |
| 8.10 | state devuelve state, track_id, position_ms, volume | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_playback_state | 2026-07-15 | no |

## 9. Cola

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 9.1 | Player expone GET /queue | NOT_TESTED | — | — | — | **sí** |
| 9.2 | Player acepta POST /queue/items | NOT_TESTED | — | — | — | **sí** |
| 9.3 | Micro Server expone GET /queue | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_queue | 2026-07-15 | **sí** |
| 9.4 | Micro Server acepta POST /queue/items | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_queue_items | 2026-07-15 | **sí** |
| 9.5 | Micro Server acepta POST /queue/jump | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_queue_jump | 2026-07-15 | no |
| 9.6 | Mobile puede agregar a cola en Player | NOT_TESTED | — | Escenario E2E (no dedicated scenario in SCENARIOS.md) | — | **sí** |
| 9.7 | Mobile puede agregar a cola en Micro Server | NOT_TESTED | — | Escenario E2E (no dedicated scenario in SCENARIOS.md) | — | **sí** |

## 10. Importación (Player → Micro Server)

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 10.1 | Player puede iniciar POST /import/session | NOT_TESTED | — | — | — | **sí** |
| 10.2 | Micro Server acepta POST /import/session | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_import_session | 2026-07-15 | **sí** |
| 10.3 | Player puede subir track POST /import/upload/:session_id | NOT_TESTED | — | — | — | **sí** |
| 10.4 | Micro Server acepta upload | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_import_upload | 2026-07-15 | **sí** |
| 10.5 | Player puede confirmar POST /import/commit/:session_id | NOT_TESTED | — | — | — | **sí** |
| 10.6 | Micro Server acepta commit | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_import_commit | 2026-07-15 | **sí** |
| 10.7 | Micro Server escanea y actualiza biblioteca post-import | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_import_commit_library_updated | 2026-07-15 | **sí** |
| 10.8 | Player → Micro Server import end-to-end funcional | NOT_TESTED | — | Escenario E2E-07 | — | **sí** |

## 11. Errores

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 11.1 | Todos los errores usan { error: { code, message, details } } | UNIT_PASS | crates/michi-api/src/routes/v1/*.rs | test_v1_error_format_includes_details | 2026-07-15 | no |
| 11.2 | Error 400 INVALID_REQUEST con details | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_error_format_includes_details | 2026-07-15 | no |
| 11.3 | Error 401 UNAUTHORIZED | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_unauthorized | 2026-07-15 | no |
| 11.4 | Error 403 FORBIDDEN | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_forbidden | 2026-07-15 | no |
| 11.5 | Error 404 NOT_FOUND / TRACK_NOT_FOUND | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_track_not_found_error | 2026-07-15 | no |
| 11.6 | Error 416 RANGE_NOT_SATISFIABLE | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_stream_range_not_satisfiable | 2026-07-15 | no |
| 11.7 | Error 429 RATE_LIMITED | NOT_TESTED | — | — | — | no |
| 11.8 | Error 500 INTERNAL_ERROR | UNIT_PASS | crates/michi-api/tests/api.rs | test_v1_internal_error | 2026-07-15 | no |

## 12. Seguridad

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 12.1 | Tokens opacos (no JWT) en el contrato | CONTRACT_PASS | schemas/pair-confirm-response.schema.json | token opaque, minLength ≥ 1 | 2026-08-05 | no |
| 12.2 | Refresh token es separado del device token | CONTRACT_PASS | schemas/pair-confirm-response.schema.json | refresh_token campo opcional independiente de token | 2026-08-05 | no |
| 12.3 | Revoke elimina ambos tokens | UNIT_PASS | crates/michi-api/src/routes/v1/pair.rs | link_devices_revoke llama revoke_all_by_device | 2026-07-15 | no |
| 12.4 | La sesión de pairing expira a los 5 min | RUST_REFERENCE_PASS | crates/michi-identity/src/pairing.rs | MAX_SESSION_DURATION (5 min) + test_expired_session | 2026-08-05 | no |
| 12.5 | No se expone file_path en respuestas públicas | UNIT_PASS | crates/michi-api/src/routes/v1/tracks.rs | file_path excluido de serialización | 2026-07-15 | no |
| 12.6 | Path traversal protegido en stream/download | UNIT_PASS | docs/DOWNSTREAM_MIGRATION.md + crates/michi-api/src/routes/v1/stream.rs (repo michi-micro-server) | validate_track_path verifica | 2026-07-15 | no |
| 12.7 | Player protege tokens en memoria | NOT_TESTED | — | — | — | no |

## 13. Mobile UX

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 13.1 | Mobile detecta servidores vía mDNS/UDP | NOT_TESTED | — | — | — | **sí** |
| 13.2 | Mobile muestra server/info.name al usuario | NOT_TESTED | — | — | — | no |
| 13.3 | Mobile adapta flujo de pairing según auth.strategy | NOT_TESTED | — | — | — | **sí** |
| 13.4 | Mobile tolera token_refresh=false | NOT_TESTED | — | — | — | **sí** |
| 13.5 | Mobile muestra error legible en fallos de red | NOT_TESTED | — | — | — | no |
| 13.6 | Mobile refresca token antes de expirar (si aplica) | NOT_TESTED | — | — | — | **sí** |

## 14. Micro Server — Reproducción Autónoma

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 14.1 | Micro Server puede reproducir sin Player conectado | UNIT_PASS | crates/michi-api/src/routes/v1/playback.rs | test_v1_playback_autonomous | 2026-07-15 | **sí** |
| 14.2 | Micro Server mantiene estado de reproducción en DB | UNIT_PASS | crates/michi-core/src/models.rs | PlaybackSessionDb con estado | 2026-07-15 | no |
| 14.3 | Micro Server puede continuar reproducción tras reinicio | NOT_TESTED | — | — | — | no |

## 15. Music Stream — Receiver v1-lite Gate

> Contrato congelado (ADR-0001) y publicado como bundle `contracts/receiver-v1-lite/` (`1.0.0-alpha.1`). Los ítems 15.1–15.6 están certificados **MOCK_PASS** contra el simulador oficial de Stream (CI cruzado run `31852348701`, 37/37 checks; SHAs mergeados `michi-link=e70c9d2014bf12e10f263339731292dc6f93624e`, `michi-music-stream=09b50d2150268eeb2ff51a37b971d0e346cbf2a4`). El ítem 15.7 (audio en hardware real) sigue **NOT_TESTED**: no hay hardware certificado ni compatibilidad física certificada.

| # | Ítem | Certification Level | Evidence | Test command | Last verified | Blocking |
|---|------|--------|----------|-------------|---------------|----------|
| 15.1 | Stream implementa `GET /server/info` con el perfil exacto (`service`, identidad, `roles: ["audio_receiver"]`, `auth.strategy: RECEIVER_BUTTON`, `audio` reproducible) | MOCK_PASS | CI run `31852348701`, check `server_info` + reporte `tests/e2e_certification/results/stream-interop-alpha1.json` | `gh run view 31852348701` (workflow `stream-interop.yml`) | 2026-08-14 | no |
| 15.2 | Stream implementa pairing canónico (ventana física 120 s; `POST /pair/start` → `GET /pair/status` → `POST /pair/confirm`; token emitido por el receptor, `expires_in: 0`) | MOCK_PASS | CI run `31852348701`, checks `pair_start`/`pair_status_*`/`pair_confirm` (+ replay 409) | `gh run view 31852348701` | 2026-08-14 | no |
| 15.3 | Stream implementa `POST /receiver-lite/session` (una sesión, RTP/UDP, PT 97, SSRC negociado, puerto 49152..65535, IP RTP = IP TCP del request) | MOCK_PASS | CI run `31852348701`, checks `session_create` + `rtp_100_packets` + `rtp_guard_rejection_classes` | `gh run view 31852348701` | 2026-08-14 | no |
| 15.4 | Stream implementa `GET/PATCH/DELETE /receiver-lite/session` (estado, volumen/pausa, cierre seguro idempotente) | MOCK_PASS | CI run `31852348701`, checks `session_get_state`/`patch_volume`/`patch_pause`/`patch_resume`/`session_delete`/`session_get_after_delete` | `gh run view 31852348701` | 2026-08-14 | no |
| 15.5 | Stream implementa `POST /receiver-lite/heartbeat` (secuencia estrictamente creciente, lease 30 s monotónico, replay 409) | MOCK_PASS | CI run `31852348701`, checks `heartbeat_*` + `lease_expire` + `session_get_after_lease_expiry` | `gh run view 31852348701` | 2026-08-14 | no |
| 15.6 | Micro Server puede crear sesión en Stream contra el contrato canónico | MOCK_PASS | CI run `31852348701` — escenario canónico E2E-09 (37/37, discovery firmado incluido), reporte `tests/e2e_certification/results/stream-interop-alpha1.json` | MS-09: E2E Micro ↔ Stream Simulator (CI `stream-interop.yml`) | 2026-08-14 | **sí** |
| 15.7 | Audio se escucha en hardware real | NOT_TESTED | — | MS-11: matriz física de certificación | — | no |

**Micro ↔ Stream Simulator: MOCK_PASS (simulador).** El pase `MOCK_PASS` (MS-09) quedó demostrado en el run `31852348701`. El pase en hardware (`DEVICE_E2E_PASS`, MS-11) sigue pendiente: hoy no existe evidencia física.
