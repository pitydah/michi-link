# Track Identity — Michi Link API v1.0.0-alpha

## Propósito

Cada track en el ecosistema Michi puede existir en múltiples dispositivos. Para evitar duplicados y permitir handoff limpio entre Player y Micro Server, necesitamos un identificador de track unificado y un mecanismo de matching por niveles de precisión.

## MichiTrackIdentity

```json
{
  "local_track_id": "track_001",
  "remote_track_id": null,
  "quick_hash": "a1b2c3d4",
  "content_hash": "sha256:a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
  "sha256_prefix": "a1b2c3d4",
  "file_size": 42345678,
  "duration_ms": 245000,
  "musicbrainz_track_id": "mbid-00000000-0000-0000-0000-000000000000",
  "normalized_title": "neon lights",
  "normalized_artist": "luna swift",
  "normalized_album": "imaginary cities"
}
```

### Campos

| Campo | Tipo | Obligatorio | Descripción |
|-------|------|-------------|-------------|
| `local_track_id` | string | Sí | ID del track en el dispositivo origen. |
| `remote_track_id` | string\|null | No | ID del track en el servidor destino, si ya existe. |
| `quick_hash` | string | No | Hash rápido (primeros 8 chars de SHA-256 o CRC32). Matching rápido sin leer archivo completo. |
| `content_hash` | string | No | SHA-256 completo del contenido del archivo (formato `sha256:...`). Matching exacto. |
| `sha256_prefix` | string | No | **Legacy.** Primeros 8 caracteres del SHA-256. Compatibilidad con implementaciones que no soportan `quick_hash`. Deprecado. |
| `file_size` | integer | No | Tamaño en bytes. Matching rápido complementario. |
| `duration_ms` | integer | No | Duración en milisegundos. Fallback de matching. |
| `musicbrainz_track_id` | string | No | MusicBrainz ID si está disponible. |
| `normalized_title` | string | No | Título normalizado (minúsculas, sin puntuación) para fuzzy matching. |
| `normalized_artist` | string | No | Artista normalizado. |
| `normalized_album` | string | No | Álbum normalizado. |

## TrackMatchResult

```json
{
  "match": "exact_hash",
  "confidence": 1.0,
  "local_track_id": "track_001",
  "remote_track_id": "server_uuid_abc",
  "already_present": true
}
```

### Campos

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `match` | string | Tipo de coincidencia (`exact_hash`, `quick_hash`, `metadata_duration`, `none`). |
| `confidence` | float | 0.0 a 1.0. |
| `local_track_id` | string | ID del track en el origen. |
| `remote_track_id` | string\|null | ID del track en el servidor, si existe. |
| `already_present` | bool | Si el track ya existe en el servidor destino. |

### Tipos de match

| match | Confianza | Condición |
|-------|-----------|-----------|
| `exact_hash` | 1.0 | `content_hash` completo coincide |
| `quick_hash` | 0.95 | `quick_hash` o `sha256_prefix` coinciden y `file_size` también |
| `metadata_duration` | 0.7 | `normalized_title + normalized_artist` coinciden y `duration_ms ± 3s` |
| `none` | 0.0 | No se encontró coincidencia |

### Orden de resolución

1. **exact_hash** — Si `content_hash` completo coincide, es el mismo track.
2. **quick_hash** — Si `quick_hash` o `sha256_prefix` + `file_size` coinciden, muy probablemente es el mismo.
3. **metadata_duration** — Si título+artista normalizados y duración coinciden aproximadamente.
4. **none** — No hay coincidencia, se debe subir como track nuevo.

## Uso en el ecosistema

| Proyecto | Uso |
|----------|-----|
| Michi Music Player | Genera `local_track_id` y `quick_hash`/`content_hash` al preparar una importación. |
| Michi Micro Server | Usa TrackIdentityResolver en preflight y upload para matching. Responde con `remote_track_id`. |
| Michi Music Mobile | Consume `remote_track_id` para mantener referencias locales estables. |
