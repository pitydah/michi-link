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
| E2E-03: Mobile ↔ Player playback | NETWORK_E2E_PASS | NOT_TESTED |
| —: Mobile ↔ Player stream (no dedicated scenario in SCENARIOS.md) | NETWORK_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/mobile_player_*.json`

### 2. Mobile ↔ Micro Server

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-04: Mobile ↔ Micro pairing | NETWORK_E2E_PASS | NOT_TESTED |
| E2E-05: Mobile ↔ Micro download/sync | NETWORK_E2E_PASS | NOT_TESTED |
| —: Mobile ↔ Micro playback (no dedicated scenario in SCENARIOS.md) | NETWORK_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/mobile_micro_*.json`

### 3. Player → Micro Server Import

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-07: Player → Micro import | LOCAL_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/player_micro_import_*.json`

### 4. Continue on Server

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| —: Continue on Server (import + queue + playback; no dedicated scenario in SCENARIOS.md, import covered by E2E-07) | LOCAL_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/continue_on_server_*.json`

### 5. Micro ↔ Stream Simulator

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-09: Micro ↔ Stream pairing + session | MOCK_PASS | MOCK_PASS |

**Evidencia:** Reporte determinista `tests/e2e_certification/results/stream-interop-alpha1.json` — CI cruzado GitHub Actions run `31852348701` (verde), 37/37 checks contra el simulador oficial de Stream. SHAs mergeados: `michi-link=e70c9d2014bf12e10f263339731292dc6f93624e`, `michi-music-stream=09b50d2150268eeb2ff51a37b971d0e346cbf2a4`. Cubre discovery firmado, pairing canónico, sesión RTP, heartbeat/lease, volumen/pausa y cierre seguro.

> **Micro ↔ Stream Simulator: MOCK_PASS (simulador).** El flujo canónico está certificado contra el simulador oficial (run `31852348701`). La certificación **física** (hardware real de Stream) sigue **NOT_TESTED**: no afirmar compatibilidad física certificada ni production ready.

### 6. Micro Autonomous Playback

| Escenario | Mínimo requerido | Estado actual |
|-----------|-----------------|---------------|
| E2E-08: Micro autonomous playback | LOCAL_E2E_PASS | NOT_TESTED |

**Evidencia:** Reporte en `tests/e2e_certification/reports/micro_autonomous_playback_*.json`

### 7. Seguridad

| Condición | Mínimo requerido | Estado | Evidencia |
|-----------|-----------------|--------|-----------|
| No file_path en respuestas públicas | UNIT_PASS | CONTRACT_PASS | schemas/track.schema.json + tests/contract (negativos de paths) |
| Error format con `{ error: { code, message, details } }` | UNIT_PASS | CONTRACT_PASS | schemas/error.schema.json + tests/contract |
| Tokens opacos (no JWT) | CONTRACT_PASS | CONTRACT_PASS | schemas/pair-confirm-response.schema.json |
| Tokens no expuestos en logs | LOCAL_E2E_PASS | NOT_TESTED | — |
| Rollback import probado | LOCAL_E2E_PASS | NOT_TESTED | — |
| Continue-on-Server fallback (Player retoma local si Micro falla) | LOCAL_E2E_PASS | NOT_TESTED | — |

### 8. Contrato

| Condición | Mínimo requerido | Estado | Evidencia |
|-----------|-----------------|--------|-----------|
| service enum correcto | UNIT_PASS | CONTRACT_PASS | tests/contract: 248 checks (negativos de service enum: rechaza servicios y aliases retirados) |
| api_version v1/v1-lite estricto | UNIT_PASS | CONTRACT_PASS | tests/contract: 248 checks (negativos: `api_version: "1.0.0"` rechazado; campo extra `michi_link_version` rechazado por `additionalProperties: false`) | <!-- michi-policy:exclude -->
| features booleanas | UNIT_PASS | CONTRACT_PASS | tests/contract: 248 checks |
| auth.required obligatorio | UNIT_PASS | CONTRACT_PASS | tests/contract: 248 checks |
| sync-delta con cursor | UNIT_PASS | CONTRACT_PASS | tests/contract: 248 checks |
| playback-control con command | UNIT_PASS | CONTRACT_PASS | tests/contract: 248 checks (negativo: rechaza `action`) |
| error format con details | UNIT_PASS | CONTRACT_PASS | tests/contract: 248 checks |
| identity ed25519-blake3-v1 (michi_id 43 chars) | UNIT_PASS | CONTRACT_PASS | tests/identity_contract: 22 checks |
| Wire base64url estricto (43/86, sin padding, sin `+`/`/`/`=`) | UNIT_PASS | CONTRACT_PASS | tests/identity_contract: 22 checks (negativos de padding) |
| Pairing canónico (challenge + session + PIN) | UNIT_PASS | RUST_REFERENCE_PASS | crates/michi-identity/src/pairing.rs |

### 8.1 Evidencia de contrato verificada (2026-08-13)

| Check | Nivel | Resultado |
|-------|-------|-----------|
| tests/contract (`npm test`) | CONTRACT_PASS | 248 checks PASS, 0 failed |
| tests/identity_contract (`npm test`) | CONTRACT_PASS | 22 checks PASS, 0 failed |
| tests/cross_layer (`npm test`) | CROSS_LAYER_PASS | 84 checks PASS, 0 failed |
| cargo test (crates/michi-identity) | RUST_REFERENCE_PASS | 88 PASS |
| clippy (crates/michi-identity) | RUST_REFERENCE_PASS | 0 warnings |
| redocly lint (openapi/michi-link-v1.yaml) | CONTRACT_PASS | 0 errors |
| python3 scripts/contract-policy.py | CONTRACT_PASS | PASS (0 findings) |
| bundle-reproducibility (CI) | BUNDLE_PASS | regenera `contracts/receiver-v1-lite/` byte a byte, sin diff |

> Evidencia E2E vigente: el escenario canónico **Micro ↔ Stream Simulator (E2E-09) está MOCK_PASS** — reporte `tests/e2e_certification/results/stream-interop-alpha1.json` (run `31852348701`, 37/37, SHAs mergeados `e70c9d2014bf12e10f263339731292dc6f93624e` + `09b50d2150268eeb2ff51a37b971d0e346cbf2a4`). El resto de la evidencia E2E sigue vacía: `tests/e2e_certification/reports/` no contiene reportes locales. La beta permanece **CERRADA** — no hay LOCAL_E2E_PASS / NETWORK_E2E_PASS / DEVICE_E2E_PASS — hasta lograr `NETWORK_E2E_PASS`/`DEVICE_E2E_PASS` en Mobile↔Player y Mobile↔Micro, y al menos `LOCAL_E2E_PASS` en Player→Micro import y Micro autonomous playback.

## Resumen

```
Condición                     Mínimo           Actual        ¿Gate passed?
──────────────────────────────────────────────────────────────────────────
Mobile ↔ Player               NETWORK_E2E_PASS NOT_TESTED     ❌
Mobile ↔ Micro                NETWORK_E2E_PASS NOT_TESTED     ❌
Player → Micro import         LOCAL_E2E_PASS   NOT_TESTED     ❌
Continue on Server            LOCAL_E2E_PASS   NOT_TESTED     ❌
Micro autonomous playback     LOCAL_E2E_PASS   NOT_TESTED     ❌
Micro ↔ Stream Simulator      MOCK_PASS        MOCK_PASS      ✅
Seguridad                     UNIT_PASS        CONTRACT_PASS  ✅
Contrato                      UNIT_PASS        CONTRACT_PASS  ✅
Reference implementation      RUST_REFERENCE_PASS  RUST_REFERENCE_PASS ✅
Cross-layer                   CROSS_LAYER_PASS  CROSS_LAYER_PASS ✅
Bundle reproducible           BUNDLE_PASS      BUNDLE_PASS    ✅
──────────────────────────────────────────────────────────────────────────
Beta gate overall:            ❌ CERRADA
```

## ¿Cómo abrir la beta gate?

1. Ejecutar cada escenario con `runner.py` en el nivel requerido.
2. Subir reportes a `tests/e2e_certification/reports/`.
3. Actualizar `BETA_READINESS_CHECKLIST.md` con los niveles alcanzados.
4. Cuando todos los escenarios cumplan el mínimo, abrir PR para declarar beta.

## Postergados para v1.0.0-beta (no bloquean alpha)

- Music Stream physical hardware (NOT_TESTED; el pase contra el simulador quedó certificado MOCK_PASS en el run `31852348701` — MS-09).
- WebSocket events (stub/partial, baja prioridad).
- Rooms/Multiroom (planned).
- Capability probing (future v1.1).
