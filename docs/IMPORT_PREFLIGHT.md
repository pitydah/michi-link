# Import Preflight — Michi Link API v1.0.0-alpha

## Propósito

Antes de iniciar una importación completa, el Player puede consultar al Micro Server qué tracks ya existen, cuáles deben subirse y cuáles tienen conflictos.

## Endpoint: POST /api/v1/import/preflight

**Auth:** Bearer token con permiso `library.write`.

### Request

```json
{
  "tracks": [
    {
      "local_track_id": "track_001",
      "quick_hash": "a1b2c3d4",
      "content_hash": "sha256:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
      "file_size": 42345678,
      "duration_ms": 245000,
      "title": "Neon Lights",
      "artist": "Luna Swift",
      "album": "Imaginary Cities"
    },
    {
      "local_track_id": "track_002",
      "quick_hash": "e5f6a7b8",
      "file_size": 31200000,
      "duration_ms": 198000,
      "title": "Pulse Wave",
      "artist": "Neon Pulse",
      "album": "Neon Dreams"
    }
  ]
}
```

### Response

```json
{
  "preflight_id": "uuid-preflight",
  "total": 2,
  "results": [
    {
      "local_track_id": "track_001",
      "status": "already_present",
      "remote_track_id": "server_track_uuid_abc",
      "match": "exact_hash"
    },
    {
      "local_track_id": "track_002",
      "status": "needs_upload",
      "remote_track_id": null,
      "match": "none"
    }
  ]
}
```

### Estados por track

| status | Significado | Acción |
|--------|-------------|--------|
| `already_present` | El track ya existe en el servidor | Saltar upload |
| `needs_upload` | El track no existe | Incluir en `POST /api/v1/import/track/upload` |
| `conflict` | Coincidencia ambigua | Revisar manualmente |

### Tipos de match en respuesta

| match | Confianza | Significado |
|-------|-----------|-------------|
| `exact_hash` | 1.0 | Coincidencia exacta de contenido |
| `quick_hash` | 0.95 | Hash rápido + tamaño coinciden |
| `metadata_duration` | 0.7 | Metadatos + duración aproximada |
| `none` | 0.0 | No encontrado |
