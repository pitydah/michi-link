# Player ↔ Micro Server Compatibility Report — Michi Link API v1.0.0-alpha

> **Nota:** Este documento define un reporte de **compatibilidad contractual** (qué endpoints declara y responde un servidor), NO es una certificación E2E. La certificación E2E usa los niveles de evidencia de `docs/BETA_READINESS_CHECKLIST.md` y los escenarios de `tests/e2e_certification/SCENARIOS.md`.

## Propósito

Cuando el Player se conecta a un Micro Server, debe evaluar qué nivel de compatibilidad tiene. No todos los servidores implementan todas las características del contrato. Este documento define cómo se reporta esa compatibilidad y qué acciones tomar según el nivel.

## Niveles de compatibilidad

| Nivel | Código | Significado |
|-------|--------|-------------|
| Contrato completo | `CONTRACT_OK` | El servidor implementa todos los endpoints requeridos. |
| Contrato parcial | `CONTRACT_PARTIAL` | El servidor implementa los endpoints críticos pero falta alguno secundario. |
| Contrato incompatible | `CONTRACT_MISMATCH` | El servidor responde pero con formato de datos diferente al esperado. |
| Endpoint faltante | `ENDPOINT_MISSING` | Un endpoint requerido devuelve 404 o no está montado. |
| Fallback disponible | `FALLBACK_AVAILABLE` | El endpoint faltante tiene un mecanismo alternativo (legacy). |

## Endpoints evaluados

| Endpoint | Requerido para CONTRACT_OK | Default si falta |
|----------|---------------------------|------------------|
| `POST /api/v1/import/session` | Sí | No hay fallback. |
| `POST /api/v1/import/upload/{session_id}` | Sí | No hay fallback. |
| `POST /api/v1/import/commit/{session_id}` | Sí | No hay fallback. Si falta, no se puede completar importación. |
| `POST /api/v1/import/preflight` | No (optimización) | Usar flujo directo `session → upload → commit` (ver docs/IMPORT_PREFLIGHT.md) |
| `POST /api/v1/import/track/upload` | No (solo flujo preflight) | Usar `POST /api/v1/import/upload/{session_id}` |
| `POST /api/v1/queue/transfer` | No (recomendado) | Usar `POST /queue/items` + `POST /queue/jump` |
| `POST /api/v1/playback/control` | Sí | No hay fallback. |
| `GET /api/v1/playback/state` | Sí | No hay fallback. |
| `POST /api/v1/token/refresh` | Sí (si auth.token_refresh=true) | No hay fallback. |

**Nota:** El flujo oficial de importación es `POST /api/v1/import/session` → `POST /api/v1/import/upload/{session_id}` → `POST /api/v1/import/commit/{session_id}` (escenario E2E-07 en `tests/e2e_certification/SCENARIOS.md`). El flujo preflight (`/import/preflight` + `/import/track/upload`) está documentado en `docs/IMPORT_PREFLIGHT.md` como optimización opcional.

## Reporte de diagnóstico

```json
{
  "server_id": "uuid-servidor",
  "name": "Michi Micro Server",
  "version": "0.1.0",
  "api_version": "v1",
  "timestamp": "2026-07-15T14:00:00Z",
  "overall": "CONTRACT_PARTIAL",
  "checks": [
    {
      "endpoint": "/api/v1/import/session",
      "method": "POST",
      "status": "CONTRACT_OK",
      "detail": "Responde con session_id + expires_at",
      "fallback": null
    },
    {
      "endpoint": "/api/v1/import/upload/{session_id}",
      "method": "POST",
      "status": "CONTRACT_OK",
      "detail": "Devuelve track_id por chunk",
      "fallback": null
    },
    {
      "endpoint": "/api/v1/import/commit/{session_id}",
      "method": "POST",
      "status": "CONTRACT_OK",
      "detail": "Devuelve tracks_imported + mapping",
      "fallback": null
    },
    {
      "endpoint": "/api/v1/queue/transfer",
      "method": "POST",
      "status": "ENDPOINT_MISSING",
      "detail": "404 Not Found",
      "fallback": "FALLBACK_AVAILABLE",
      "fallback_detail": "Usar POST /queue/items + POST /queue/jump"
    },
    {
      "endpoint": "/api/v1/playback/state",
      "method": "GET",
      "status": "CONTRACT_OK",
      "detail": "Devuelve state: playing/paused/stopped + position_ms",
      "fallback": null
    },
    {
      "endpoint": "/api/v1/playback/control",
      "method": "POST",
      "status": "CONTRACT_OK",
      "detail": "Acepta command + position_ms + volume",
      "fallback": null
    }
  ],
  "warnings": [
    "queue/transfer no disponible, se usará fallback legacy"
  ],
  "mismatches": []
}
```

### Campos del reporte

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `server_id` | string | ID del servidor evaluado. |
| `name` | string | Nombre del servidor (campo canónico `name` de server-info). |
| `version` | string | Versión de la aplicación (campo canónico `version`; no es la versión del contrato). |
| `api_version` | string | Versión del contrato (`"v1"`). |
| `timestamp` | string | ISO 8601. |
| `overall` | string | `CONTRACT_OK`, `CONTRACT_PARTIAL`, `CONTRACT_MISMATCH`. |
| `checks[]` | array | Lista de verificaciones individuales. |
| `checks[].status` | string | `CONTRACT_OK`, `CONTRACT_MISMATCH`, `ENDPOINT_MISSING`. |
| `checks[].fallback` | string\|null | `FALLBACK_AVAILABLE` o null. |
| `warnings[]` | array | Advertencias no críticas. |
| `mismatches[]` | array | Incompatibilidades de formato detectadas. |

## Cómo se genera

El Player ejecuta estos pasos al conectarse a un Micro Server:

1. `GET /api/v1/server/info` → Obtiene `service`, `auth.strategy`, `api_version`, `version`.
2. Para cada endpoint de la tabla, hace un request de prueba:
   - Si responde 200/201/204 → `CONTRACT_OK`
   - Si responde 404 → `ENDPOINT_MISSING`
   - Si responde 400/500 con formato inesperado → `CONTRACT_MISMATCH`
3. Si el response tiene estructura diferente al schema esperado → `CONTRACT_MISMATCH`
4. Calcula `overall`:
   - Todos `CONTRACT_OK` → `CONTRACT_OK`
   - Críticos `CONTRACT_OK` + no críticos `ENDPOINT_MISSING` con fallback → `CONTRACT_PARTIAL`
   - Algún crítico `ENDPOINT_MISSING` sin fallback o `CONTRACT_MISMATCH` → `CONTRACT_MISMATCH`

## Acciones según nivel

| overall | Qué hace el Player |
|---------|-------------------|
| `CONTRACT_OK` | Continua con Continue-on-Server completo (session → upload → commit → queue transfer → play). |
| `CONTRACT_PARTIAL` | Continua con Continue-on-Server usando fallbacks (legacy queue). Muestra advertencia al usuario. |
| `CONTRACT_MISMATCH` | No continua. Muestra error al usuario. Ofrece reintentar. |
