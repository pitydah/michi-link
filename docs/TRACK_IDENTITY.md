# Track Identity — Michi Link API v1.0.0-alpha

## Propósito

Cada track en el ecosistema Michi puede existir en múltiples dispositivos. Para evitar duplicados y permitir handoff limpio entre Player y Micro Server, necesitamos un identificador de track unificado y un mecanismo de matching.

## MichiTrackIdentity

```json
{
  "michi_track_id": "uuid-unificado",
  "content_hash": "sha256:a1b2c3d4...",
  "file_size": 42345678,
  "duration_ms": 245000,
  "musicbrainz_track_id": "mbid-00000000-0000-0000-0000-000000000000",
  "normalized_title": "neon lights",
  "normalized_artist": "luna swift",
  "normalized_album": "imaginary cities",
  "source_device": "michi-music-player",
  "source_track_id": "track_001"
}
```

### Campos

| Campo | Tipo | Obligatorio | Descripción |
|-------|------|-------------|-------------|
| `michi_track_id` | string (UUID) | Sí | Identificador unificado del track en el ecosistema. Se genera en el origen y se preserva al migrar. |
| `content_hash` | string | No | SHA-256 del contenido del archivo. Permite matching exacto sin importar metadatos. |
| `file_size` | integer | No | Tamaño en bytes. Útil para matching rápido. |
| `duration_ms` | integer | No | Duración en milisegundos. Útil como fallback de matching. |
| `musicbrainz_track_id` | string | No | MusicBrainz ID si está disponible. |
| `normalized_title` | string | No | Título normalizado (minúsculas, sin puntuación) para fuzzy matching. |
| `normalized_artist` | string | No | Artista normalizado. |
| `normalized_album` | string | No | Álbum normalizado. |
| `source_device` | string | Sí | Dispositivo que originó el track. |
| `source_track_id` | string | Sí | ID del track en el dispositivo origen. |

## TrackMatchResult

```json
{
  "michi_track_id": "uuid-unificado",
  "match_type": "exact_hash",
  "confidence": 1.0,
  "already_on_server": false,
  "server_track_id": null
}
```

### match_type

| Valor | Significado | Confianza |
|-------|-------------|-----------|
| `exact_hash` | Coincidencia exacta de contenido (SHA-256) | 1.0 |
| `metadata_match` | Coincidencia por título+artista+álbum normalizados | 0.9 |
| `duration_match` | Coincidencia por duración + artista + título aproximado | 0.6 |
| `conflict` | Múltiples coincidencias posibles | 0.0 |
| `not_found` | No se encontró coincidencia | 0.0 |

### Campos

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `michi_track_id` | string | ID unificado (existente o nuevo) |
| `match_type` | string | Tipo de coincidencia |
| `confidence` | float | 0.0 a 1.0 |
| `already_on_server` | bool | Si el track ya existe en el servidor destino |
| `server_track_id` | string\|null | ID del track en el servidor destino si ya existe |
| `conflicts` | array | Lista de IDs en conflicto si match_type = conflict |

## TrackIdentityResolver

El resolver es la lógica que implementa este matching. No es un endpoint separado, es parte del flujo de import preflight.

### Orden de resolución

1. **exact_hash** — Si `content_hash` coincide, es el mismo track.
2. **musicbrainz_track_id** — Si ambos tracks tienen el mismo MBID.
3. **metadata_match** — Si `normalized_title + normalized_artist + normalized_album` coinciden.
4. **duration_match** — Si duración ±2s y artista+album son muy similares.
5. **not_found** — No hay coincidencia, se debe subir como track nuevo.

## Uso en el ecosistema

| Proyecto | Uso |
|----------|-----|
| Michi Music Player | Genera `michi_track_id` al importar música por primera vez. Calcula `content_hash` y normaliza metadatos. |
| Michi Micro Server | Usa TrackIdentityResolver durante import preflight para decidir si aceptar, saltar o marcar conflicto. |
| Michi Music Mobile | Consume `michi_track_id` para mantener referencias locales estables al sincronizar. |
