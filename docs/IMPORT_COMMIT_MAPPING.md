# Import Commit Mapping — Michi Link API v1.0.0-alpha

## Propósito

Cuando finaliza una sesión de importación, el commit debe consolidar y devolver el mapping completo de todos los tracks procesados: tanto los que se subieron como los que ya existían.

## Endpoint: POST /api/v1/import/commit/{session_id}

**Auth:** Bearer token con permiso `library.write`.

### Response

```json
{
  "session_id": "uuid-sesion",
  "tracks_imported": 2,
  "tracks_skipped": 1,
  "total_tracks": 3,
  "mapping": [
    {
      "local_track_id": "track_001",
      "remote_track_id": "server_track_uuid_abc",
      "status": "uploaded",
      "match": "exact_hash"
    },
    {
      "local_track_id": "track_002",
      "remote_track_id": "server_track_uuid_def",
      "status": "uploaded",
      "match": "quick_hash"
    },
    {
      "local_track_id": "track_003",
      "remote_track_id": "server_track_existing_xyz",
      "status": "already_present",
      "match": "exact_hash"
    }
  ]
}
```

### Campos

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `session_id` | string | ID de la sesión de importación. |
| `tracks_imported` | int | Tracks nuevos importados. |
| `tracks_skipped` | int | Tracks que ya existían (duplicados). |
| `total_tracks` | int | Suma de importados + skippeados. |
| `mapping[]` | array | Lista completa de mappings. |

### mapping[]

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `local_track_id` | string | ID del track en el dispositivo origen. |
| `remote_track_id` | string | ID del track en el servidor destino. |
| `status` | string | `uploaded` (nuevo), `already_present` (existente). |
| `match` | string | Tipo de matching usado: `exact_hash`, `quick_hash`, `metadata_duration`, `none`. |

### Uso

El Player usa `mapping[]` para:

1. Construir la cola remota con `remote_track_id`.
2. Conservar la correspondencia para futuras operaciones.
3. Saber qué tracks se saltaron (ya existían).

### Compatibilidad con el formato legacy

El formato anterior usaba `mapping` como objeto (`{ local_id: remote_id }`). Ese formato sigue siendo aceptado en el response, pero el nuevo formato `mapping[]` como array es el oficial. El servidor puede devolver ambos durante la transición.
