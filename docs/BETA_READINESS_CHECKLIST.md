# Beta Readiness Checklist — Michi Link API v1.0.0-alpha

## Instrucciones

Cada ítem debe estar en estado **PASS** antes de declarar la beta. Si algún ítem está **FAIL** o **NOT_TESTED**, la beta no puede comenzar.

---

## 1. Contrato

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 1.1 | server-info.schema.json service enum tiene `michi-music-player` | PASS | michi-link | |
| 1.2 | server-info.schema.json features son booleanos | PASS | michi-link | |
| 1.3 | server-info.schema.json auth.required es obligatorio | PASS | michi-link | |
| 1.4 | server-info.schema.json michi_link_version es string | PASS | michi-link | |
| 1.5 | playback-control.schema.json requiere command | PASS | michi-link | |
| 1.6 | sync-delta.schema.json usa cursor (string) | PASS | michi-link | |
| 1.7 | error.schema.json usa { error: { code, message, details } } | PASS | michi-link | |
| 1.8 | Ejemplos validan contra schemas (31 tests) | PASS | michi-link | |
| 1.9 | OpenAPI spec actualizada | PASS | michi-link | |

## 2. Autenticación

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 2.1 | Player expone `/server/info` con auth.strategy=PLAYER_PASSWORD | NOT_TESTED | michi-player | |
| 2.2 | Micro Server expone `/server/info` con auth.strategy=SERVER_CODE | PASS | michi-micro-server | |
| 2.3 | Player token_refresh=false | NOT_TESTED | michi-player | |
| 2.4 | Micro Server token_refresh=true | PASS | michi-micro-server | |
| 2.5 | Mobile detecta estrategia y actúa en consecuencia | NOT_TESTED | michi-mobile | |
| 2.6 | Receiver expone auth.strategy=RECEIVER_BUTTON | NOT_TESTED | michi-stream | |

## 3. Pairing

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 3.1 | Player acepta POST /pair/start | NOT_TESTED | michi-player | |
| 3.2 | Player acepta POST /pair/confirm | NOT_TESTED | michi-player | |
| 3.3 | Micro Server acepta POST /pair/start | PASS | michi-micro-server | |
| 3.4 | Micro Server acepta POST /pair/confirm | PASS | michi-micro-server | |
| 3.5 | Mobile puede pairar con Player | NOT_TESTED | michi-mobile | Beta blocker |
| 3.6 | Mobile puede pairar con Micro Server | NOT_TESTED | michi-mobile | Beta blocker |
| 3.7 | Pairing code expira (5 min) | PASS | michi-micro-server | |
| 3.8 | Pairing confirm devuelve token + permisos canónicos | PASS | michi-micro-server | |

## 4. Biblioteca

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 4.1 | Player expone GET /tracks | NOT_TESTED | michi-player | |
| 4.2 | Micro Server expone GET /tracks | PASS | michi-micro-server | |
| 4.3 | Player expone GET /albums | NOT_TESTED | michi-player | |
| 4.4 | Micro Server expone GET /albums | PASS | michi-micro-server | |
| 4.5 | Player expone GET /search | NOT_TESTED | michi-player | |
| 4.6 | Micro Server expone GET /search | PASS | michi-micro-server | |
| 4.7 | Paginación funciona (page, limit, total, has_more) | PASS | michi-micro-server | |
| 4.8 | Filtros por artista, álbum, género, año | PASS | michi-micro-server | |

## 5. Streaming

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 5.1 | Player sirve GET /stream/{id} con Range | NOT_TESTED | michi-player | |
| 5.2 | Micro Server sirve GET /stream/{id} con Range | PASS | michi-micro-server | |
| 5.3 | Responde 206 Partial Content con Range válido | PASS | michi-micro-server | |
| 5.4 | Responde 416 RANGE_NOT_SATISFIABLE con Range inválido | PASS | michi-micro-server | |
| 5.5 | Responde 404 TRACK_NOT_FOUND si track no existe | PASS | michi-micro-server | |
| 5.6 | Responde 200 OK sin Range | PASS | michi-micro-server | |
| 5.7 | Transcoding on-the-fly (mp3, ogg, hls) | partial | michi-micro-server | |

## 6. Descargas

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 6.1 | Micro Server sirve GET /download/{id} | PASS | michi-micro-server | |
| 6.2 | Requiere permiso download.read | PASS | michi-micro-server | |
| 6.3 | Responde Content-Disposition: attachment | PASS | michi-micro-server | |
| 6.4 | Player implementa GET /download | NOT_TESTED | michi-player | planned |
| 6.5 | Mobile puede descargar desde Micro Server | NOT_TESTED | michi-mobile | Beta blocker |

## 7. Sincronización

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 7.1 | Micro Server expone GET /sync/manifest | PASS | michi-micro-server | |
| 7.2 | Micro Server expone GET /sync/manifest/delta?cursor= | PASS | michi-micro-server | |
| 7.3 | Micro Server acepta POST /sync/state | PASS | michi-micro-server | |
| 7.4 | Manifest incluye cursor | PASS | michi-micro-server | |
| 7.5 | Delta acepta cursor, since y manifest_id (legacy) | PASS | michi-micro-server | |
| 7.6 | Player expone sync endpoints | NOT_TESTED | michi-player | |
| 7.7 | Mobile puede sincronizar desde Micro Server | NOT_TESTED | michi-mobile | Beta blocker |

## 8. Reproducción

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 8.1 | Player expone GET /playback/state | NOT_TESTED | michi-player | |
| 8.2 | Player acepta POST /playback/control con command | NOT_TESTED | michi-player | |
| 8.3 | Micro Server expone GET /playback/state | PASS | michi-micro-server | |
| 8.4 | Micro Server acepta POST /playback/control con command | PASS | michi-micro-server | |
| 8.5 | Micro Server acepta action como legacy | PASS | michi-micro-server | |
| 8.6 | seek usa position_ms | PASS | michi-micro-server | |
| 8.7 | set_volume usa volume 0-100 | PASS | michi-micro-server | |
| 8.8 | Mobile puede controlar Player | NOT_TESTED | michi-mobile | Beta blocker |
| 8.9 | Mobile puede controlar Micro Server | NOT_TESTED | michi-mobile | Beta blocker |
| 8.10 | state devuelve state, track_id, position_ms, volume | PASS | michi-micro-server | |

## 9. Cola

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 9.1 | Player expone GET /queue | NOT_TESTED | michi-player | |
| 9.2 | Player acepta POST /queue/items | NOT_TESTED | michi-player | |
| 9.3 | Micro Server expone GET /queue | PASS | michi-micro-server | |
| 9.4 | Micro Server acepta POST /queue/items | PASS | michi-micro-server | |
| 9.5 | Micro Server acepta POST /queue/jump | PASS | michi-micro-server | |
| 9.6 | Mobile puede agregar a cola en Player | NOT_TESTED | michi-mobile | Beta blocker |
| 9.7 | Mobile puede agregar a cola en Micro Server | NOT_TESTED | michi-mobile | Beta blocker |

## 10. Importación (Player → Micro Server)

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 10.1 | Player puede iniciar POST /import/session | NOT_TESTED | michi-player | |
| 10.2 | Micro Server acepta POST /import/session | PASS | michi-micro-server | |
| 10.3 | Player puede subir track POST /import/upload/:session_id | NOT_TESTED | michi-player | |
| 10.4 | Micro Server acepta upload | PASS | michi-micro-server | |
| 10.5 | Player puede confirmar POST /import/commit/:session_id | NOT_TESTED | michi-player | |
| 10.6 | Micro Server acepta commit | PASS | michi-micro-server | |
| 10.7 | Micro Server escanea y actualiza biblioteca post-import | PASS | michi-micro-server | |
| 10.8 | Player → Micro Server import end-to-end funcional | NOT_TESTED | ambos | Beta blocker |

## 11. Errores

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 11.1 | Todos los errores usan { error: { code, message, details } } | PASS | michi-micro-server | |
| 11.2 | Error 400 INVALID_REQUEST con details | PASS | michi-micro-server | |
| 11.3 | Error 401 UNAUTHORIZED | PASS | michi-micro-server | |
| 11.4 | Error 403 FORBIDDEN | PASS | michi-micro-server | |
| 11.5 | Error 404 NOT_FOUND / TRACK_NOT_FOUND | PASS | michi-micro-server | |
| 11.6 | Error 416 RANGE_NOT_SATISFIABLE | PASS | michi-micro-server | |
| 11.7 | Error 429 RATE_LIMITED | NOT_TESTED | ambos | |
| 11.8 | Error 500 INTERNAL_ERROR | PASS | michi-micro-server | |

## 12. Seguridad

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 12.1 | Tokens hasheados (SHA-256) en store | PASS | michi-micro-server | |
| 12.2 | Refresh token es separado del device token | PASS | michi-micro-server | |
| 12.3 | Revoke elimina ambos tokens | PASS | michi-micro-server | |
| 12.4 | Pairing code expira a los 5 min | PASS | michi-micro-server | |
| 12.5 | No se expone file_path en respuestas públicas | PASS | michi-micro-server | |
| 12.6 | Path traversal protegido en stream/download | PASS | michi-micro-server | |
| 12.7 | Player protege tokens en memoria | NOT_TESTED | michi-player | |

## 13. Mobile UX

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 13.1 | Mobile detecta servidores vía mDNS/UDP | NOT_TESTED | michi-mobile | |
| 13.2 | Mobile muestra server/info.name al usuario | NOT_TESTED | michi-mobile | |
| 13.3 | Mobile adapta flujo de pairing según auth.strategy | NOT_TESTED | michi-mobile | |
| 13.4 | Mobile tolera token_refresh=false | NOT_TESTED | michi-mobile | |
| 13.5 | Mobile muestra error legible en fallos de red | NOT_TESTED | michi-mobile | |
| 13.6 | Mobile refresca token antes de expirar (si aplica) | NOT_TESTED | michi-mobile | |

## 14. Micro Server — Reproducción Autónoma

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 14.1 | Micro Server puede reproducir sin Player conectado | partial | michi-micro-server | |
| 14.2 | Micro Server mantiene estado de reproducción en DB | partial | michi-micro-server | |
| 14.3 | Micro Server puede continuar reproducción tras reinicio | partial | michi-micro-server | |

## 15. Music Stream — Prototype Gate

| # | Ítem | Estado | Responsable | Notas |
|---|------|--------|-------------|-------|
| 15.1 | Firmware Stream implementa receiver/info | NOT_TESTED | michi-stream | prototype |
| 15.2 | Firmware Stream implementa receiver/pair/start | NOT_TESTED | michi-stream | prototype |
| 15.3 | Firmware Stream implementa receiver/heartbeat | NOT_TESTED | michi-stream | prototype |
| 15.4 | Firmware Stream implementa receiver/session/start | NOT_TESTED | michi-stream | prototype |
| 15.5 | Firmware Stream recibe y reproduce URL de stream | NOT_TESTED | michi-stream | prototype |
| 15.6 | Micro Server puede enviar sesión a Stream | NOT_TESTED | michi-micro-server | |
| 15.7 | Audio se escucha en hardware real | NOT_TESTED | michi-stream | Gate para beta |
