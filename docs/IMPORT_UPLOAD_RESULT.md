# Import Upload Result — Michi Link API v1.0.0-alpha

## Propósito

Cada upload individual devuelve un resultado que el Player usa para saber si el track se subió correctamente, ya existía o no pudo confirmarse el mapping.

## Endpoint: POST /api/v1/import/track/upload

**Auth:** Bearer token con permiso `library.write`.

### Response exitoso

```json
{
  "local_track_id": "track_001",
  "remote_track_id": "server_track_uuid_abc",
  "status": "uploaded",
  "checksum": "sha256:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
}
```

### Response duplicado

```json
{
  "local_track_id": "track_002",
  "remote_track_id": "server_track_existing_uuid",
  "status": "already_present",
  "checksum": "sha256:b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
}
```

### Response con MAPPING_UNCONFIRMED

Cuando el servidor acepta el archivo pero no puede devolver un `remote_track_id` inmediatamente (ej: procesamiento asíncrono, archivo en cola de análisis):

```json
{
  "local_track_id": "track_003",
  "remote_track_id": null,
  "status": "accepted",
  "checksum": "sha256:c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
  "warning": "MAPPING_UNCONFIRMED"
}
```

### Campos

| Campo | Tipo | Obligatorio | Descripción |
|-------|------|-------------|-------------|
| `local_track_id` | string | Sí | Eco del ID enviado en el request. |
| `remote_track_id` | string\|null | Sí | UUID asignado por el servidor, o `null` si no se pudo confirmar. |
| `status` | string | Sí | `uploaded`, `already_present` o `accepted`. |
| `checksum` | string | No | Checksum calculado por el servidor para verificación. |
| `warning` | string | No | `MAPPING_UNCONFIRMED` si el mapping remoto no está disponible. |

### Comportamiento del Player ante MAPPING_UNCONFIRMED

1. El Player almacena el track localmente como `MAPPING_UNCONFIRMED`.
2. El Player debe esperar al commit para obtener el mapping definitivo.
3. Si el commit tampoco devuelve mapping para ese `local_track_id`, el Player debe marcar el track como `MAPPING_FAILED` y no incluirlo en operaciones posteriores.

### Estados de mapping en el Player

| Estado | Significado |
|--------|-------------|
| `MAPPING_CONFIRMED` | `remote_track_id` conocido. Se puede usar en queue/control. |
| `MAPPING_UNCONFIRMED` | Track subido pero sin `remote_track_id`. Esperar commit. |
| `MAPPING_FAILED` | Ni upload ni commit devolvieron mapping. No incluir en operaciones. |

### Diferencia con IMPORT_UPLOAD_MAPPING.md

| Aspecto | IMPORT_UPLOAD_MAPPING.md | IMPORT_UPLOAD_RESULT.md (este doc) |
|---------|--------------------------|-------------------------------------|
| Enfoque | Flujo general de upload + mapping | Formato exacto del response + estados |
| MAPPING_UNCONFIRMED | No mencionado | Documentado con warning |
| Estados de mapping | No definidos | CONFIRMED / UNCONFIRMED / FAILED |
