# Continue on Server Contract — Michi Link API v1.0.0-alpha

## Propósito

Permitir que Michi Music Player transfiera su sesión de reproducción actual a Michi Micro Server para que la música siga sonando cuando el PC se apaga.

## Flujo Completo

```
Player                              Micro Server
  │                                      │
  │  1. Resolver identidad de tracks     │
  │  ──────────────────────────────────> │
  │  POST /api/v1/import/preflight       │
  │  <────────────────────────────────── │
  │                                      │
  │  2. Subir tracks faltantes           │
  │  ──────────────────────────────────> │
  │  POST /api/v1/import/upload/{id}     │
  │  (repetir por cada track faltante)   │
  │  <────────────────────────────────── │
  │                                      │
  │  3. Confirmar importación            │
  │  ──────────────────────────────────> │
  │  POST /api/v1/import/commit/{id}     │
  │  <────────────────────────────────── │
  │                                      │
  │  4. Sincronizar cola                 │
  │  ──────────────────────────────────> │
  │  POST /api/v1/queue/items            │
  │  (tracks en orden)                   │
  │  <────────────────────────────────── │
  │                                      │
  │  5. Posicionar cola                  │
  │  ──────────────────────────────────> │
  │  POST /api/v1/queue/jump             │
  │  { index: current_index }            │
  │  <────────────────────────────────── │
  │                                      │
  │  6. Enviar estado de reproducción    │
  │  ──────────────────────────────────> │
  │  POST /api/v1/sync/state             │
  │  { track_id, position_ms, playing }  │
  │  <────────────────────────────────── │
  │                                      │
  │  7. Iniciar reproducción remota      │
  │  ──────────────────────────────────> │
  │  POST /api/v1/playback/control       │
  │  { command: "play" }                 │
  │  <────────────────────────────────── │
  │                                      │
  │  8. Confirmar estado                 │
  │  ──────────────────────────────────> │
  │  GET /api/v1/playback/state          │
  │  <─────────────── state: playing     │
  │                                      │
  │  9. Pausar reproducción local        │
  │  (Player detiene su salida de audio) │
  │                                      │
```

## Contrato por paso

### Paso 1: Preflight (opcional pero recomendado)

**Endpoint:** `POST /api/v1/import/preflight`

Ver docs/IMPORT_PREFLIGHT.md. Si no se usa, pasar directo a paso 2.

### Paso 2: Subir tracks

**Endpoint:** `POST /api/v1/import/upload/{session_id}`

No necesita `michi_track_id`. El servidor puede asignar uno. Si el servidor detecta duplicado por hash, responde `is_duplicate: true`.

### Paso 3: Confirmar importación

**Endpoint:** `POST /api/v1/import/commit/{session_id}`

Response incluye `tracks_imported` y `tracks_skipped` (duplicados).

```json
{
  "session_id": "uuid-sesion",
  "tracks_imported": 5,
  "tracks_skipped": 3,
  "total_tracks": 8,
  "mapping": {
    "source_track_001": "server_track_001",
    "source_track_002": "server_track_002"
  },
  "michi_track_ids": {
    "server_track_001": "michi_track_uuid_001"
  }
}
```

### Paso 4: Sincronizar cola

**Endpoint:** `POST /api/v1/queue/items`

El Player envía los `track_ids` (IDs del servidor obtenidos del mapping en commit) en orden.

### Paso 5: Posicionar cola

**Endpoint:** `POST /api/v1/queue/jump`

```json
{
  "index": 2
}
```

### Paso 6: Enviar estado

**Endpoint:** `POST /api/v1/sync/state`

```json
{
  "track_id": "server_track_003",
  "position_ms": 45000,
  "playing": true,
  "volume": 0.8
}
```

### Paso 7: Iniciar reproducción

**Endpoint:** `POST /api/v1/playback/control`

```json
{
  "command": "play",
  "position_ms": 45000
}
```

### Paso 8: Confirmar

**Endpoint:** `GET /api/v1/playback/state`

### Paso 9: Pausa local

El Player detiene su salida de audio local. No es un endpoint, es lógica interna del Player.

## Rollback

Si algún paso falla:

| Paso fallido | Acción de rollback |
|-------------|-------------------|
| 2 (upload) | Reintentar upload individual. Si persiste, marcar track como failed y continuar. |
| 3 (commit) | No hay tracks en la biblioteca aún. No requiere rollback. |
| 4 (queue) | La cola no se modificó. Reintentar. |
| 5 (jump) | La cola existe pero no se movió. Reintentar. |
| 6 (state) | El estado no persiste. Reintentar. |
| 7 (play) | El servidor no empezó a reproducir. Player retoma local. |

Si el paso 7 falla después de que el Player ya pausó su salida local:
1. Player reanuda reproducción local.
2. Reporta error.
3. No se requiere limpieza en el servidor.

## Requisitos

- Player y Micro Server deben estar emparejados (token con permisos `library.write`, `playback.control`, `queue.write`).
- Micro Server debe tener capacidad de reproducción autónoma (no requiere Player como fuente).
- La red debe permitir al Player alcanzar al Micro Server.
