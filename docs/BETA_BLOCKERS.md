# Beta Blockers — Michi Link API v1.0.0-alpha.1

## Definición

Un **beta blocker** es un flujo E2E que debe funcionar correctamente antes de declarar la versión beta del ecosistema Michi.

## Tabla de Bloqueadores

| # | Bloqueador | Descripción | Proyectos | Dependencias | Riesgo |
|---|-----------|-------------|-----------|--------------|--------|
| B-01 | Mobile ↔ Player pairing | Mobile descubre Player, inicia pairing, recibe token | Mobile, Player | Player expone pair/start + pair/confirm | Alto |
| B-02 | Mobile ↔ Player stream | Mobile reproduce música desde Player vía HTTP Range | Mobile, Player | Player expone stream/{id} con Range | Alto |
| B-03 | Mobile ↔ Player playback | Mobile envía play/pause/seek/volume a Player | Mobile, Player | Player acepta playback/control con command | Alto |
| B-04 | Mobile ↔ Micro pairing | Mobile descubre Micro, inicia pairing, recibe token + refresh | Mobile, Micro Server | Micro expone pair/start + pair/confirm + auth.strategy=SERVER_CODE | Alto |
| B-05 | Mobile ↔ Micro download | Mobile descarga track desde Micro para offline | Mobile, Micro Server | Micro expone download/{id} con permiso download.read | Alto |
| B-06 | Mobile ↔ Micro playback | Mobile envía play/pause/seek/volume a Micro | Mobile, Micro Server | Micro acepta playback/control con command | Alto |
| B-07 | Player → Micro import | Player envía biblioteca completa a Micro Server | Player, Micro Server | Micro expone import/session + upload + commit | Alto |
| B-08 | Micro autonomous playback | Micro reproduce sin Player conectado | Micro Server | Micro mantiene estado en DB y puede continuar tras reinicio | Alto |
| B-09 | Micro ↔ Stream pairing | Micro inicia pairing `RECEIVER_BUTTON` contra el receptor durante su ventana física de 120 s y recibe el token emitido por el receptor | Micro, Stream | Stream implementa el flujo canónico `/pair/start` → `/pair/status` → `/pair/confirm` del perfil v1-lite | Medio |
| B-10 | Micro ↔ Stream session | Micro crea la sesión RTP/UDP PCM (`POST /receiver-lite/session`) en Stream y la mantiene con heartbeat | Micro, Stream | Ambos implementan la superficie canónica `receiver-lite/session` + `receiver-lite/heartbeat` del bundle | Medio |
| B-11 | Reportes E2E vacíos | `tests/e2e_certification/reports/` no contiene ningún reporte | Todos | Correr escenarios con runner.py y subir reportes | Alto |
| B-12 | Sin NETWORK_E2E_PASS | Ningún escenario E2E certificado en LAN real (todo NOT_TESTED) | Todos | Ejecutar E2E en red local real | Alto |

## Estado Actual

| # | Estado | Responsable | ETA estimada |
|---|--------|-------------|--------------|
| B-01 | NOT_TESTED | michi-music-player, michi-music-mobile | TBD |
| B-02 | NOT_TESTED | michi-music-player, michi-music-mobile | TBD |
| B-03 | NOT_TESTED | michi-music-player, michi-music-mobile | TBD |
| B-04 | NOT_TESTED | michi-micro-server, michi-music-mobile | TBD |
| B-05 | NOT_TESTED | michi-micro-server, michi-music-mobile | TBD |
| B-06 | NOT_TESTED | michi-micro-server, michi-music-mobile | TBD |
| B-07 | NOT_TESTED | michi-music-player, michi-micro-server | TBD |
| B-08 | NOT_TESTED | michi-micro-server | TBD |
| B-09 | NOT_TESTED (Micro ↔ Stream Simulator: NOT_TESTED — el pairing v1-lite aún no se certifica contra el simulador de Stream) | michi-micro-server, michi-music-stream | Stream debe implementar el pairing canónico del bundle v1-lite; Micro debe consumirlo con `device_type: "server"` |
| B-10 | NOT_TESTED (Micro ↔ Stream Simulator: NOT_TESTED — la sesión RTP/UDP aún no se certifica contra el simulador de Stream) | michi-micro-server, michi-music-stream | Stream debe implementar la sesión canónica `receiver-lite/session` (una sesión, RTP/UDP, PT 97, lease por heartbeat) |
| B-11 | NOT_TESTED (reports/ vacío) | Todos | Correr escenarios y certificar |
| B-12 | NOT_TESTED | Todos | Ejecutar en LAN real |

## Criterio de Desbloqueo

Para cada bloqueador:
1. Reporte E2E certificado (según formato E2E_CERTIFICATION.md).
2. Sin errores críticos en los checks.
3. Ejecutado en red local real (`NETWORK_E2E_PASS`) o hardware real (`DEVICE_E2E_PASS`).

Cuando todos los bloqueadores B-01 a B-08 estén en estado PASS **y** `tests/e2e_certification/reports/` contenga los reportes correspondientes (B-11, B-12), se puede declarar beta.
