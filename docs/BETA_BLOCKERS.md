# Beta Blockers — Michi Link API v1.0.0-alpha

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
| B-09 | Micro ↔ Stream pairing | Stream físico se empareja con Micro Server | Micro, Stream | Stream implementa v1-lite pairing | Medio |
| B-10 | Micro ↔ Stream session | Micro envía stream URL a Stream y reproducción comienza | Micro, Stream | Ambos implementan session/start + heartbeat | Medio |

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
| B-08 | PARTIAL | michi-micro-server | TBD |
| B-09 | NOT_TESTED | michi-micro-server, michi-music-stream | TBD |
| B-10 | NOT_TESTED | michi-micro-server, michi-music-stream | TBD |

## Criterio de Desbloqueo

Para cada bloqueador:
1. Reporte E2E certificado (según formato E2E_CERTIFICATION.md).
2. Sin errores críticos en los checks.
3. Ejecutado en red local real.

Cuando todos los bloqueadores B-01 a B-08 estén en estado PASS, se puede declarar beta.
