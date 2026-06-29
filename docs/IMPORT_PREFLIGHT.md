# Import Preflight — Michi Link API v1.0.0-alpha

## Propósito

Antes de iniciar una importación completa, el Player puede consultar al Micro Server qué tracks ya existen, cuáles deben subirse y cuáles tienen conflictos.

## Endpoint: POST /api/v1/import/preflight

**Estado:** Alpha (opcional para un import funcional, pero recomendado para evitar duplicados).

**Auth:** Bearer token con permiso `library.write`.

### Request

```json
{
  "tracks": [
    {
      "michi_track_id": "uuid-origen-001",
      "content_hash": "sha256:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
      "file_size": 42345678,
      "duration_ms": 245000,
      "musicbrainz_track_id": "mbid-00000000-0000-0000-0000-000000000000",
      "title": "Neon Lights",
      "artist": "Luna Swift",
      "album": "Imaginary Cities",
      "album_artist": "Luna Swift",
      "track_number": 1,
      "disc_number": 1,
      "year": 2025,
      "genre": "Electronic",
      "format": "flac",
      "bitrate": 1411,
      "sample_rate": 44100,
      "channels": 2
    }
  ]
}
```

### Response

```json
{
  "preflight_id": "uuid-preflight",
  "total": 10,
  "already_present": 3,
  "needs_upload": 6,
  "conflicts": 1,
  "results": [
    {
      "index": 0,
      "status": "already_present",
      "michi_track_id": "uuid-unificado",
      "server_track_id": "uuid-servidor",
      "match_type": "exact_hash",
      "confidence": 1.0
    },
    {
      "index": 1,
      "status": "needs_upload",
      "michi_track_id": "uuid-origen-002",
      "match_type": "not_found",
      "confidence": 0.0
    },
    {
      "index": 2,
      "status": "conflict",
      "michi_track_id": "uuid-origen-003",
      "match_type": "duration_match",
      "confidence": 0.6,
      "conflicting_tracks": [
        { "server_track_id": "uuid-servidor-003a", "title": "Neon Lights (Remix)", "confidence": 0.6 }
      ],
      "resolution": "skip"
    }
  ]
}
```

### Estados por track

| Estado | Significado | Acción recomendada |
|--------|-------------|-------------------|
| `already_present` | El track ya existe en el servidor | Saltar upload |
| `needs_upload` | El track no existe, debe subirse | Incluir en import/upload |
| `conflict` | Coincidencia ambigua | Revisar manualmente o resolver por regla |

### Resolución automática de conflictos

Si `resolution` no se especifica en el response, el cliente puede usar estas reglas:

1. Si hay un `exact_hash` match, siempre es `already_present`.
2. Si hay un `metadata_match` con confianza ≥ 0.9 y solo un candidato, es `already_present`.
3. Si hay múltiples candidatos o confianza < 0.9, es `conflict`.

## Integración con import existente

El preflight es opcional pero altamente recomendado. Si no se usa:

- El Micro Server puede detectar duplicados por `content_hash` durante import/upload.
- Si detecta duplicado, responde `is_duplicate: true` y `track_id` del existente.
- El Player decide si sobrescribe o salta.
