# Michi Link Beta Gate

## Definición

La **beta gate** es el conjunto de condiciones que deben cumplirse antes de declarar oficialmente la versión beta del ecosistema Michi.

## Condiciones

### 1. Mobile ↔ Player E2E_PASS

| Escenario | Nivel requerido | Estado actual |
|-----------|----------------|---------------|
| E2E-01: Mobile ↔ Player pairing | E2E_PASS | NOT_TESTED |
| E2E-02: Mobile ↔ Player stream | E2E_PASS | NOT_TESTED |
| E2E-03: Mobile ↔ Player playback | E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/mobile_player_*.json`

### 2. Mobile ↔ Micro Server E2E_PASS

| Escenario | Nivel requerido | Estado actual |
|-----------|----------------|---------------|
| E2E-04: Mobile ↔ Micro pairing | E2E_PASS | NOT_TESTED |
| E2E-05: Mobile ↔ Micro download/sync | E2E_PASS | NOT_TESTED |
| E2E-06: Mobile ↔ Micro playback | E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/mobile_micro_*.json`

### 3. Player → Micro Server Import E2E_PASS

| Escenario | Nivel requerido | Estado actual |
|-----------|----------------|---------------|
| E2E-07: Player → Micro import | E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/player_micro_import_*.json`

### 4. Micro Autonomous Playback E2E_PASS

| Escenario | Nivel requerido | Estado actual |
|-----------|----------------|---------------|
| E2E-08: Micro autonomous playback | E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/micro_autonomous_playback_*.json`

### 5. Seguridad

| Condición | Estado | Evidencia |
|-----------|--------|-----------|
| No file_path en respuestas públicas | PASS | test_v1_tracks_no_file_path |
| Error format con { error: { code, message, details } } | PASS | test_v1_error_format_includes_details |
| Tokens hasheados (SHA-256) | PASS | crates/michi-link/src/auth.rs |
| Tokens no expuestos en logs | NOT_TESTED | — |

### 6. Contrato

| Condición | Estado | Evidencia |
|-----------|--------|-----------|
| service enum correcto | PASS | 31/31 tests de contrato |
| features booleanas | PASS | 31/31 tests de contrato |
| auth.required obligatorio | PASS | 31/31 tests de contrato |
| michi_link_version string | PASS | 31/31 tests de contrato |
| sync-delta con cursor | PASS | 31/31 tests de contrato |
| playback-control con command | PASS | 31/31 tests de contrato |
| error format con details | PASS | 31/31 tests de contrato |

## Resumen

```
Condición                     Requerido    Actual     ¿Gate passed?
──────────────────────────────────────────────────────────────
Mobile ↔ Player               E2E_PASS     NOT_TESTED  ❌
Mobile ↔ Micro                E2E_PASS     NOT_TESTED  ❌
Player → Micro import         E2E_PASS     NOT_TESTED  ❌
Micro autonomous playback     E2E_PASS     NOT_TESTED  ❌
Seguridad                     PASS         PASS        ✅
Contrato                      PASS         PASS        ✅
──────────────────────────────────────────────────────────────
Beta gate overall:            ❌ CERRADA
```

## ¿Cómo abrir la beta gate?

1. Ejecutar cada escenario con `runner.py`.
2. Subir reportes a `tests/e2e_certification/reports/`.
3. Actualizar `BETA_READINESS_CHECKLIST.md` con los niveles alcanzados.
4. Cuando todos los escenarios E2E_PASS estén en ✅, abrir PR para declarar beta.

## Postergados para v1.0.0-beta (no bloquean alpha)

- Music Stream physical hardware (prototype).
- WebSocket events (stub/partial, baja prioridad).
- Rooms/Multiroom (planned).
- Capability probing (future v1.1).
