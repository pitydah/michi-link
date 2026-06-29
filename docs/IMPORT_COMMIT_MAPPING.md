# Import Commit Mapping — Michi Link API v1.0.0-alpha

## Propósito

Cuando el Player importa tracks al Micro Server, el servidor asigna nuevos IDs locales. El Player necesita saber la correspondencia entre sus IDs locales y los IDs remotos para poder referenciar esos tracks en operaciones posteriores (queue, playback control, continue-on-server).

## El problema

Player tiene `local_track_id` (ej: `track_001`). Micro Server asigna `remote_track_id` (ej: `a1b2c3d4-...`). Sin mapping, el Player no puede decirle al Micro Server "reproduce el track que acabo de subir".

## La solución

`POST /api/v1/import/commit/{session_id}` debe devolver un mapping completo.

### Response

```json
{
  "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "tracks_imported": 3,
  "tracks_skipped": 1,
  "total_tracks": 4,
  "mapping": {
    "local_track_001": "remote_track_uuid_abc",
    "local_track_002": "remote_track_uuid_def",
    "local_track_003": "remote_track_uuid_ghi"
  },
  "michi_track_ids": {
    "remote_track_uuid_abc": "michi_track_uuid_001",
    "remote_track_uuid_def": "michi_track_uuid_002",
    "remote_track_uuid_ghi": "michi_track_uuid_003"
  },
  "skipped_mapping": {
    "local_track_004": "remote_track_uuid_existing"
  }
}
```

### Campos

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `session_id` | UUID | ID de la sesión de importación |
| `tracks_imported` | int | Tracks nuevos importados |
| `tracks_skipped` | int | Tracks que ya existían (duplicados) |
| `total_tracks` | int | Suma de importados + skippeados |
| `mapping` | object | Clave = `local_track_id` (enviado durante upload), Valor = `remote_track_id` (asignado por servidor). Solo tracks nuevos. |
| `michi_track_ids` | object | Clave = `remote_track_id`, Valor = `michi_track_id` unificado. |
| `skipped_mapping` | object | Clave = `local_track_id`, Valor = `remote_track_id` existente. Solo tracks duplicados. |

## Cómo se usa

Después de commit, el Player puede:

1. Leer `mapping` para saber qué IDs remotos corresponden a sus tracks locales.
2. Usar esos IDs remotos para construir la cola (`POST /api/v1/queue/items`).
3. Usar `michi_track_ids` para mantener referencias estables entre sesiones.

## Cómo genera el servidor el mapping

El servidor debe almacenar temporalmente la correspondencia entre `track_index` (enviado en upload) y `remote_track_id` (asignado al procesar el upload). Al hacer commit, consolida el mapping.

### Formato de upload

Cada `POST /api/v1/import/upload/{session_id}` debe incluir:

```json
{
  "track_index": 0,
  "local_track_id": "local_track_001",
  "filename": "neon_lights.flac",
  "hash": "sha256:a1b2c3d4..."
}
```

El servidor almacena:

```
session_id → [(track_index=0, local_track_id="local_track_001", remote_track_id="remote_uuid_abc"), ...]
```

Al hacer commit, construye `mapping` a partir de esa tabla temporal.

## Ejemplo de flujo completo

```
Player: POST /import/preflight
        → Recibe: needs_upload=[track_001, track_002]

Player: POST /import/upload/{session} x2
        → body: { track_index:0, local_track_id:"track_001", filename:"..." }
        → body: { track_index:1, local_track_id:"track_002", filename:"..." }

Player: POST /import/commit/{session}
        → Recibe: mapping { "track_001": "remote_abc", "track_002": "remote_def" }

Player: POST /queue/items
        → body: { track_ids: ["remote_abc", "remote_def"] }
        → Funciona porque ahora conoce los IDs remotos.
```
