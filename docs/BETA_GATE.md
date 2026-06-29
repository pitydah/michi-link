# Michi Link Beta Gate

## Definición

La **beta gate** es el conjunto de condiciones que deben cumplirse antes de declarar oficialmente la versión beta del ecosistema Michi.

## Niveles requeridos por escenario

| Escenario | Mínimo para beta | Ideal |
|-----------|-----------------|-------|
| Mobile ↔ Player pairing | **NETWORK_E2E_PASS** | DEVICE_E2E_PASS |
| Mobile ↔ Player stream | **NETWORK_E2E_PASS** | DEVICE_E2E_PASS |
| Mobile ↔ Player playback | **NETWORK_E2E_PASS** | DEVICE_E2E_PASS |
| Mobile ↔ Micro pairing | **NETWORK_E2E_PASS** | DEVICE_E2E_PASS |
| Mobile ↔ Micro download | **NETWORK_E2E_PASS** | DEVICE_E2E_PASS |
| Mobile ↔ Micro playback | **NETWORK_E2E_PASS** | DEVICE_E2E_PASS |
| Player → Micro import | **LOCAL_E2E_PASS** | NETWORK_E2E_PASS |
| Continue on Server | **LOCAL_E2E_PASS** | NETWORK_E2E_PASS |
| Micro autonomous | **LOCAL_E2E_PASS** | NETWORK_E2E_PASS |
| Micro ↔ Stream Simulator | **MOCK_PASS** | NETWORK_E2E_PASS |

## Condiciones

### 1. Mobile ↔ Player

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-01: Mobile ↔ Player pairing | NETWORK_E2E_PASS | NOT_TESTED |
| E2E-02: Mobile ↔ Player stream | NETWORK_E2E_PASS | NOT_TESTED |
| E2E-03: Mobile ↔ Player playback | NETWORK_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/mobile_player_*.json`

### 2. Mobile ↔ Micro Server

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-04: Mobile ↔ Micro pairing | NETWORK_E2E_PASS | NOT_TESTED |
| E2E-05: Mobile ↔ Micro download/sync | NETWORK_E2E_PASS | NOT_TESTED |
| E2E-06: Mobile ↔ Micro playback | NETWORK_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/mobile_micro_*.json`

### 3. Player → Micro Server Import

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-07: Player → Micro import | LOCAL_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/player_micro_import_*.json`

### 4. Continue on Server

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-09: Continue on Server (import + queue + playback) | LOCAL_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/continue_on_server_*.json`

### 5. Micro ↔ Stream Simulator

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-10: Micro ↔ Stream pairing + session | MOCK_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/micro_stream_receiver_*.json`

### 6. Micro Autonomous Playback

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-08: Micro autonomous playback | LOCAL_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/micro_autonomous_playback_*.json`

### 7. Seguridad

| Condición | Mínimo requerido | Estado | Evidencia |
|-----------|-----------------|--------|-----------|
| No file_path en respuestas públicas | UNIT_PASS | UNIT_PASS | test_v1_tracks_no_file_path |
| Error format con `{ error: { code, message, details } }` | UNIT_PASS | UNIT_PASS | test_v1_error_format_includes_details |
| Tokens hasheados (SHA-256) | UNIT_PASS | UNIT_PASS | crates/michi-link/src/auth.rs |
| Tokens no expuestos en logs | LOCAL_E2E_PASS | NOT_TESTED | — |
| Rollback import probado | LOCAL_E2E_PASS | NOT_TESTED | — |
| Continue-on-Server fallback (Player retoma local si Micro falla) | LOCAL_E2E_PASS | NOT_TESTED | — |

### 8. Contrato

| Condición | Mínimo requerido | Estado | Evidencia |
|-----------|-----------------|--------|-----------|
| service enum correcto | UNIT_PASS | UNIT_PASS | 34 tests de contrato |
| features booleanas | UNIT_PASS | UNIT_PASS | 34 tests de contrato |
| auth.required obligatorio | UNIT_PASS | UNIT_PASS | 34 tests de contrato |
| michi_link_version string | UNIT_PASS | UNIT_PASS | 34 tests de contrato |
| sync-delta con cursor | UNIT_PASS | UNIT_PASS | 34 tests de contrato |
| playback-control con command | UNIT_PASS | UNIT_PASS | 34 tests de contrato |
| error format con details | UNIT_PASS | UNIT_PASS | 34 tests de contrato |

## Resumen

```
Condición                     Mínimo           Actual        ¿Gate passed?
──────────────────────────────────────────────────────────────────────────
Mobile ↔ Player               NETWORK_E2E_PASS NOT_TESTED     ❌
Mobile ↔ Micro                NETWORK_E2E_PASS NOT_TESTED     ❌
Player → Micro import         LOCAL_E2E_PASS   NOT_TESTED     ❌
Continue on Server            LOCAL_E2E_PASS   NOT_TESTED     ❌
Micro autonomous playback     LOCAL_E2E_PASS   NOT_TESTED     ❌
Micro ↔ Stream Simulator      MOCK_PASS        NOT_TESTED     ❌
Seguridad                     UNIT_PASS        UNIT_PASS      ✅
Contrato                      UNIT_PASS        UNIT_PASS      ✅
──────────────────────────────────────────────────────────────────────────
Beta gate overall:            ❌ CERRADA
```

## ¿Cómo abrir la beta gate?

1. Ejecutar cada escenario con `runner.py` en el nivel requerido.
2. Subir reportes a `tests/e2e_certification/reports/`.
3. Actualizar `BETA_READINESS_CHECKLIST.md` con los niveles alcanzados.
4. Cuando todos los escenarios cumplan el mínimo, abrir PR para declarar beta.

## Postergados para v1.0.0-beta (no bloquean alpha)

- Music Stream physical hardware (prototype, sigue como MOCK_PASS).
- WebSocket events (stub/partial, baja prioridad).
- Rooms/Multiroom (planned).
- Capability probing (future v1.1).
