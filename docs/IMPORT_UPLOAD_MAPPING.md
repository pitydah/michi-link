# Import Upload Mapping — Michi Link API v1.0.0-alpha

## Propósito

Cuando el Player sube un track al Micro Server, el servidor asigna un ID remoto y devuelve el mapping inmediatamente. Esto permite al Player conocer el `remote_track_id` sin esperar al commit.

## Endpoint: POST /api/v1/import/track/upload

**Auth:** Bearer token con permiso `library.write`.

**Alternativa legacy:** `POST /api/v1/import/upload/{session_id}` (no requiere `local_track_id`).

### Request

```json
{
  "local_track_id": "track_001",
  "quick_hash": "a1b2c3d4",
  "content_hash": "sha256:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
  "filename": "neon_lights.flac",
  "title": "Neon Lights",
  "artist": "Luna Swift",
  "album": "Imaginary Cities",
  "duration_ms": 245000,
  "file_size": 42345678,
  "format": "flac",
  "bitrate": 1411,
  "sample_rate": 44100,
  "channels": 2,
  "year": 2025,
  "track_number": 1,
  "disc_number": 1,
  "genre": "Electronic",
  "checksum": "sha256:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
}
```

### Response

```json
{
  "local_track_id": "track_001",
  "remote_track_id": "server_track_uuid_abc",
  "status": "uploaded",
  "checksum": "sha256:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
}
```

### Campos del response

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `local_track_id` | string | Eco del ID enviado en el request. |
| `remote_track_id` | string | UUID asignado por el servidor al track. |
| `status` | string | `uploaded` (nuevo), `already_present` (duplicado detectado). |
| `checksum` | string | Checksum calculado por el servidor para verificación. |

### Detección de duplicados

Si el servidor detecta que el track ya existe (por `content_hash`, `quick_hash` o `metadata_duration`):

```json
{
  "local_track_id": "track_001",
  "remote_track_id": "server_track_existing_uuid",
  "status": "already_present",
  "checksum": "sha256:a1b2c3d4..."
}
```

El Player debe usar `remote_track_id` existente en lugar del que esperaba crear.

### Diferencia con el endpoint legacy

| Aspecto | `/api/v1/import/upload/{session_id}` (legacy) | `/api/v1/import/track/upload` (nuevo) |
|---------|-----------------------------------------------|----------------------------------------|
| Agrupación | Requiere session_id | Sin sesión, track individual |
| local_track_id | Opcional | Requerido |
| Mapping | Solo al hacer commit | Inmediato en el response |
| Duplicados | `is_duplicate: true` | `status: already_present` |
