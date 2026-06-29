# Michi Link API v1

- **Version:** 1.0.0
- **Transport:** HTTP/1.1 (HTTPS recommended)
- **Content-Type:** `application/json`
- **Authentication:** Bearer Token via `Authorization` header
- **Base URL:** `http://{host}:{port}/api/v1`

---

## Índice

1. [Autenticación](#autenticación)
2. [Formato de errores](#formato-de-errores)
3. [Paginación](#paginación)
4. [Rate Limiting](#rate-limiting)
5. [Endpoints](#endpoints)
   - [Server](#server)
   - [Pairing](#pairing)
   - [Tokens](#tokens)
   - [Dispositivos](#dispositivos)
   - [Biblioteca](#biblioteca)
   - [Tracks](#tracks)
   - [Álbumes](#álbumes)
   - [Artistas](#artistas)
   - [Búsqueda](#búsqueda)
   - [Streaming](#streaming)
   - [Descargas](#descargas)
   - [Carátulas](#carátulas)
   - [Playlists](#playlists)
   - [Sincronización](#sincronización)
   - [Reproducción](#reproducción)
   - [Cola](#cola)
   - [Receptores](#receptores)
   - [Salas](#salas)
   - [Eventos](#eventos)
   - [Receiver Lite (v1-lite)](#receiver-lite-v1-lite)

---

## Autenticación

Todos los endpoints (excepto `/status` y los de pairing inicial) requieren un token Bearer en el header `Authorization`:

```
Authorization: Bearer <token>
```

Los tokens se obtienen mediante el flujo de pairing (`/pair/start`, `/pair/confirm`) y se renuevan vía `/token/refresh`.

---

## Formato de errores

Toda respuesta de error sigue la misma estructura:

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Descripción legible del error.",
    "details": {}
  }
}
```

| Código                     | Significado                            |
|----------------------------|----------------------------------------|
| `INVALID_REQUEST`          | La solicitud está mal formada.         |
| `UNAUTHORIZED`             | Token ausente, inválido o expirado.    |
| `FORBIDDEN`                | El token no tiene el permiso necesario.|
| `NOT_FOUND`                | Recurso no encontrado.                 |
| `CONFLICT`                 | Conflicto de estado.                   |
| `PAIRING_REQUIRED`         | El dispositivo no está emparejado.     |
| `PAIRING_IN_PROGRESS`      | Ya hay un pairing en curso.            |
| `PAIRING_EXPIRED`          | El código de pairing expiró.           |
| `RATE_LIMITED`             | Demasiadas solicitudes.                |
| `INTERNAL_ERROR`           | Error interno del servidor.            |

---

## Paginación

Los endpoints que devuelven listas aceptan los siguientes parámetros de consulta:

| Parámetro | Tipo   | Defecto | Descripción                     |
|-----------|--------|---------|----------------------------------|
| `page`    | int    | 1       | Número de página.                |
| `limit`   | int    | 50      | Elementos por página (máx 200).  |
| `sort`    | string | —       | Campo de ordenamiento.           |
| `order`   | string | `asc`   | Dirección: `asc` o `desc`.       |

Respuesta paginada:

```json
{
  "data": [],
  "page": 1,
  "limit": 50,
  "total": 243,
  "has_more": true
}
```

---

## Rate Limiting

- Límite general: **120 solicitudes por minuto** por token.
- Endpoints de streaming y descarga: **30 solicitudes por minuto** por token.
- Endpoints de pairing: **5 solicitudes por minuto** por IP.
- Ante exceso se responde con `HTTP 429` y código `RATE_LIMITED`.
- Headers de rate limit incluidos en cada respuesta:
  - `X-RateLimit-Limit`
  - `X-RateLimit-Remaining`
  - `X-RateLimit-Reset`

---

## Endpoints

---

### Server

#### `GET /server/info`

Obtiene la identidad, roles activos y capacidades del servidor.

**Respuesta `200 OK`:**

```json
{
  "server_name": "Michi Link Server",
  "server_version": "1.0.0",
  "api_version": "1.0.0",
  "device_id": "uuid-del-servidor",
  "roles": ["core", "sync_leader", "library_service"],
  "capabilities": {
    "streaming_formats": ["flac", "mp3", "ogg"],
    "max_bitrate": 320,
    "max_sample_rate": 192000,
    "multiroom": true,
    "transcoding": true,
    "sync": true
  },
  "uptime_seconds": 84720
}
```

---

### Health Check

#### `GET /status`

Endpoint público sin autenticación. Verifica que el servidor está operativo.

**Respuesta `200 OK`:**

```json
{
  "status": "ok",
  "version": "1.0.0",
  "uptime_seconds": 84720,
  "timestamp": "2026-06-29T12:00:00Z"
}
```

---

### Pairing

#### `POST /pair/start`

Inicia el flujo de emparejamiento. Un código se muestra en pantalla para que el usuario lo confirme en otro dispositivo.

**Cuerpo de solicitud:**

```json
{
  "device_name": "Mi Teléfono",
  "device_type": "mobile",
  "roles": ["controller", "player"],
  "capabilities": {
    "streaming_formats": ["mp3"],
    "max_bitrate": 320
  }
}
```

**Respuesta `200 OK`:**

```json
{
  "pairing_code": "ABCD-1234",
  "expires_in_seconds": 300,
  "device_id": "uuid-del-dispositivo",
  "pin_required": false
}
```

**Respuesta `409 Conflict`** si ya hay un pairing activo:

```json
{
  "error": {
    "code": "PAIRING_IN_PROGRESS",
    "message": "Ya existe un proceso de emparejamiento activo.",
    "details": {
      "expires_in_seconds": 180
    }
  }
}
```

---

#### `POST /pair/confirm`

Confirma el emparejamiento con el código obtenido en `/pair/start`.

**Cuerpo de solicitud:**

```json
{
  "device_id": "uuid-del-dispositivo",
  "pairing_code": "ABCD-1234"
}
```

**Respuesta `200 OK`:**

```json
{
  "token": "eyJhbGciOiJI...",
  "refresh_token": "eyJhbGciOiJI...",
  "expires_in_seconds": 3600,
  "device_id": "uuid-del-dispositivo",
  "server_id": "uuid-del-servidor"
}
```

**Respuesta `401 Unauthorized`:**

```json
{
  "error": {
    "code": "PAIRING_EXPIRED",
    "message": "El código de emparejamiento ha expirado.",
    "details": {}
  }
}
```

---

### Tokens

#### `POST /token/refresh`

Renueva el token de acceso usando el refresh token.

**Cuerpo de solicitud:**

```json
{
  "refresh_token": "eyJhbGciOiJI..."
}
```

**Respuesta `200 OK`:**

```json
{
  "token": "eyJhbGciOiJI...",
  "refresh_token": "eyJhbGciOiJI...",
  "expires_in_seconds": 3600
}
```

**Respuesta `401 Unauthorized`:**

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Refresh token inválido o expirado.",
    "details": {}
  }
}
```

---

### Dispositivos

#### `POST /devices/revoke`

Revoca el acceso de un dispositivo emparejado.

**Cuerpo de solicitud:**

```json
{
  "device_id": "uuid-del-dispositivo-a-revocar"
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "revoked_device_id": "uuid-del-dispositivo-a-revocar"
}
```

---

### Biblioteca

#### `GET /library/stats`

Obtiene estadísticas de la biblioteca de música.

**Respuesta `200 OK`:**

```json
{
  "total_tracks": 12543,
  "total_albums": 1024,
  "total_artists": 512,
  "total_playlists": 48,
  "total_duration_seconds": 3120000,
  "total_size_bytes": 274877906944,
  "last_scan": "2026-06-28T10:00:00Z",
  "scan_in_progress": false
}
```

---

#### `POST /library/scan`

Inicia un escaneo completo de la biblioteca. Es idempotente: no inicia un segundo escaneo si ya hay uno en curso.

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "scan_id": "uuid-del-escaneo",
  "started_at": "2026-06-29T12:00:00Z"
}
```

**Respuesta `409 Conflict`:**

```json
{
  "error": {
    "code": "CONFLICT",
    "message": "Ya hay un escaneo en curso.",
    "details": {
      "scan_id": "uuid-del-escaneo-actual"
    }
  }
}
```

---

### Tracks

#### `GET /tracks`

Lista o busca tracks. Soporta paginación y filtros.

**Parámetros de consulta:**

| Parámetro  | Tipo   | Descripción                            |
|------------|--------|----------------------------------------|
| `q`        | string | Término de búsqueda (nombre, artista). |
| `artist`   | string | Filtrar por ID de artista.             |
| `album`    | string | Filtrar por ID de álbum.               |
| `genre`    | string | Filtrar por género.                    |
| `year`     | int    | Filtrar por año.                       |
| `favorite` | bool   | Filtrar solo favoritos.                |

**Respuesta `200 OK`:**

```json
{
  "data": [
    {
      "id": "uuid-del-track",
      "title": "Canción de Ejemplo",
      "artist": "Artista Ejemplo",
      "artist_id": "uuid-del-artista",
      "album": "Álbum Ejemplo",
      "album_id": "uuid-del-album",
      "track_number": 3,
      "disc_number": 1,
      "duration_seconds": 245,
      "genre": "Rock",
      "year": 2024,
      "cover_id": "uuid-de-la-caratula",
      "file_format": "flac",
      "bitrate": 1411,
      "sample_rate": 44100,
      "size_bytes": 35000000,
      "favorite": false,
      "created_at": "2026-01-15T08:30:00Z"
    }
  ],
  "page": 1,
  "limit": 50,
  "total": 12543,
  "has_more": true
}
```

---

#### `GET /tracks/{id}`

Obtiene los detalles completos de un track específico.

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-del-track",
  "title": "Canción de Ejemplo",
  "artist": "Artista Ejemplo",
  "artist_id": "uuid-del-artista",
  "album": "Álbum Ejemplo",
  "album_id": "uuid-del-album",
  "track_number": 3,
  "disc_number": 1,
  "duration_seconds": 245,
  "genre": "Rock",
  "year": 2024,
  "cover_id": "uuid-de-la-caratula",
  "file_format": "flac",
  "bitrate": 1411,
  "sample_rate": 44100,
  "size_bytes": 35000000,
  "favorite": false,
  "play_count": 42,
  "last_played": "2026-06-28T18:00:00Z",
  "created_at": "2026-01-15T08:30:00Z",
  "updated_at": "2026-06-01T12:00:00Z"
}
```

**Respuesta `404 Not Found`:**

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Track no encontrado.",
    "details": {}
  }
}
```

---

### Álbumes

#### `GET /albums`

Lista o busca álbumes. Soporta paginación y filtros.

**Parámetros de consulta:**

| Parámetro | Tipo   | Descripción                          |
|-----------|--------|--------------------------------------|
| `q`       | string | Término de búsqueda (nombre, artista).|
| `artist`  | string | Filtrar por ID de artista.           |
| `genre`   | string | Filtrar por género.                  |
| `year`    | int    | Filtrar por año.                     |

**Respuesta `200 OK`:**

```json
{
  "data": [
    {
      "id": "uuid-del-album",
      "title": "Álbum Ejemplo",
      "artist": "Artista Ejemplo",
      "artist_id": "uuid-del-artista",
      "year": 2024,
      "genre": "Rock",
      "track_count": 12,
      "duration_seconds": 2940,
      "cover_id": "uuid-de-la-caratula",
      "favorite": false,
      "created_at": "2026-01-15T08:30:00Z"
    }
  ],
  "page": 1,
  "limit": 50,
  "total": 1024,
  "has_more": true
}
```

---

#### `GET /albums/{id}`

Obtiene los detalles de un álbum, incluyendo su lista de tracks.

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-del-album",
  "title": "Álbum Ejemplo",
  "artist": "Artista Ejemplo",
  "artist_id": "uuid-del-artista",
  "year": 2024,
  "genre": "Rock",
  "track_count": 12,
  "duration_seconds": 2940,
  "cover_id": "uuid-de-la-caratula",
  "favorite": false,
  "created_at": "2026-01-15T08:30:00Z",
  "tracks": [
    {
      "id": "uuid-del-track",
      "title": "Canción de Ejemplo",
      "track_number": 1,
      "duration_seconds": 245
    }
  ]
}
```

---

### Artistas

#### `GET /artists`

Lista o busca artistas. Soporta paginación y filtros.

**Parámetros de consulta:**

| Parámetro | Tipo   | Descripción                          |
|-----------|--------|--------------------------------------|
| `q`       | string | Término de búsqueda.                 |
| `genre`   | string | Filtrar por género.                  |

**Respuesta `200 OK`:**

```json
{
  "data": [
    {
      "id": "uuid-del-artista",
      "name": "Artista Ejemplo",
      "album_count": 8,
      "track_count": 96,
      "genre": "Rock",
      "cover_id": "uuid-de-la-caratula",
      "favorite": false,
      "created_at": "2026-01-15T08:30:00Z"
    }
  ],
  "page": 1,
  "limit": 50,
  "total": 512,
  "has_more": true
}
```

---

#### `GET /artists/{id}`

Obtiene los detalles de un artista, incluyendo sus álbumes y tracks destacados.

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-del-artista",
  "name": "Artista Ejemplo",
  "album_count": 8,
  "track_count": 96,
  "genre": "Rock",
  "cover_id": "uuid-de-la-caratula",
  "favorite": false,
  "bio": "Biografía del artista...",
  "created_at": "2026-01-15T08:30:00Z",
  "albums": [
    {
      "id": "uuid-del-album",
      "title": "Álbum Ejemplo",
      "year": 2024,
      "cover_id": "uuid-de-la-caratula"
    }
  ],
  "top_tracks": [
    {
      "id": "uuid-del-track",
      "title": "Canción de Ejemplo",
      "play_count": 150
    }
  ]
}
```

---

### Búsqueda

#### `GET /search`

Búsqueda global a través de tracks, álbumes, artistas y playlists.

**Parámetros de consulta:**

| Parámetro | Tipo   | Obligatorio | Descripción                     |
|-----------|--------|-------------|----------------------------------|
| `q`       | string | Sí          | Término de búsqueda.             |
| `type`    | string | No          | Filtrar: `tracks`, `albums`, `artists`, `playlists`. Por defecto busca en todos. |
| `limit`   | int    | No          | Resultados por categoría (máx 20).|

**Respuesta `200 OK`:**

```json
{
  "query": "ejemplo",
  "tracks": {
    "data": [],
    "total": 15
  },
  "albums": {
    "data": [],
    "total": 3
  },
  "artists": {
    "data": [],
    "total": 2
  },
  "playlists": {
    "data": [],
    "total": 1
  }
}
```

---

### Streaming

#### `GET /stream/{track_id}`

Transmite (stream) un track de audio. Soporta peticiones parciales vía `Range` header para búsqueda y reanudación.

**Headers de solicitud:**

| Header        | Descripción                                     |
|---------------|-------------------------------------------------|
| `Range`       | (Opcional) Rango de bytes, ej: `bytes=0-1023`.  |
| `Accept`      | Formato deseado, ej: `audio/flac`, `audio/mpeg`.|

**Respuesta `200 OK`** (sin Range):

Headers:
```
Content-Type: audio/flac
Content-Length: 35000000
Accept-Ranges: bytes
```

Cuerpo: flujo binario del archivo de audio.

**Respuesta `206 Partial Content`** (con Range):

Headers:
```
Content-Type: audio/flac
Content-Length: 1024
Content-Range: bytes 0-1023/35000000
Accept-Ranges: bytes
```

Cuerpo: porción solicitada del archivo de audio.

**Respuesta `416 Range Not Satisfiable`:**

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Rango solicitado no válido.",
    "details": {}
  }
}
```

---

### Descargas

#### `GET /download/{track_id}`

Descarga un track completo para uso offline. Autenticación requerida. No soporta Range (a diferencia de `/stream`).

**Headers de solicitud:**

| Header   | Descripción                                     |
|----------|-------------------------------------------------|
| `Accept` | Formato deseado, ej: `audio/flac`, `audio/mpeg`.|

**Respuesta `200 OK`:**

Headers:
```
Content-Type: audio/flac
Content-Length: 35000000
Content-Disposition: attachment; filename="cancion_ejemplo.flac"
```

Cuerpo: archivo binario completo.

---

### Carátulas

#### `GET /artwork/{cover_id}`

Obtiene la imagen de carátula de un álbum o track.

**Parámetros de consulta:**

| Parámetro | Tipo   | Defecto | Descripción           |
|-----------|--------|---------|-----------------------|
| `size`    | string | `large` | `small`, `medium`, `large`. |

**Respuesta `200 OK`:**

Headers:
```
Content-Type: image/jpeg
Content-Length: 128000
Cache-Control: public, max-age=86400
```

Cuerpo: datos binarios de la imagen.

---

### Playlists

#### `GET /playlists`

Lista todas las playlists del usuario.

**Respuesta `200 OK`:**

```json
{
  "data": [
    {
      "id": "uuid-de-la-playlist",
      "name": "Mis Favoritas",
      "description": "Mis canciones favoritas",
      "owner": "uuid-del-dispositivo",
      "is_public": false,
      "track_count": 25,
      "duration_seconds": 5400,
      "cover_id": "uuid-de-la-caratula",
      "created_at": "2026-03-10T14:00:00Z",
      "updated_at": "2026-06-29T10:00:00Z"
    }
  ],
  "page": 1,
  "limit": 50,
  "total": 48,
  "has_more": true
}
```

---

#### `POST /playlists`

Crea una nueva playlist.

**Cuerpo de solicitud:**

```json
{
  "name": "Nueva Playlist",
  "description": "Descripción opcional",
  "is_public": false
}
```

**Respuesta `201 Created`:**

```json
{
  "id": "uuid-de-la-playlist",
  "name": "Nueva Playlist",
  "description": "Descripción opcional",
  "owner": "uuid-del-dispositivo",
  "is_public": false,
  "track_count": 0,
  "duration_seconds": 0,
  "created_at": "2026-06-29T12:00:00Z",
  "updated_at": "2026-06-29T12:00:00Z"
}
```

---

#### `GET /playlists/{id}`

Obtiene los detalles de una playlist.

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-de-la-playlist",
  "name": "Mis Favoritas",
  "description": "Mis canciones favoritas",
  "owner": "uuid-del-dispositivo",
  "is_public": false,
  "track_count": 25,
  "duration_seconds": 5400,
  "cover_id": "uuid-de-la-caratula",
  "created_at": "2026-03-10T14:00:00Z",
  "updated_at": "2026-06-29T10:00:00Z"
}
```

---

#### `PUT /playlists/{id}`

Actualiza los metadatos de una playlist.

**Cuerpo de solicitud:**

```json
{
  "name": "Mis Favoritas (Actualizado)",
  "description": "Nueva descripción",
  "is_public": true
}
```

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-de-la-playlist",
  "name": "Mis Favoritas (Actualizado)",
  "description": "Nueva descripción",
  "is_public": true,
  "updated_at": "2026-06-29T12:05:00Z"
}
```

---

#### `DELETE /playlists/{id}`

Elimina una playlist.

**Respuesta `204 No Content`:** (sin cuerpo)

**Respuesta `404 Not Found`:**

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Playlist no encontrada.",
    "details": {}
  }
}
```

---

#### `GET /playlists/{id}/tracks`

Obtiene los tracks de una playlist, ordenados por posición.

**Respuesta `200 OK`:**

```json
{
  "data": [
    {
      "id": "uuid-del-track",
      "position": 0,
      "title": "Canción de Ejemplo",
      "artist": "Artista Ejemplo",
      "artist_id": "uuid-del-artista",
      "album": "Álbum Ejemplo",
      "album_id": "uuid-del-album",
      "duration_seconds": 245,
      "cover_id": "uuid-de-la-caratula",
      "added_at": "2026-06-29T10:00:00Z"
    }
  ],
  "total": 25
}
```

---

#### `PUT /playlists/{id}/tracks`

Reemplaza o actualiza los tracks de una playlist.

**Cuerpo de solicitud:**

```json
{
  "track_ids": ["uuid-track-1", "uuid-track-2", "uuid-track-3"]
}
```

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-de-la-playlist",
  "track_count": 3,
  "duration_seconds": 735
}
```

---

### Sincronización

#### `GET /sync/manifest`

Obtiene el manifiesto completo de sincronización. Contiene todos los elementos de la biblioteca con sus versiones.

**Respuesta `200 OK`:**

```json
{
  "manifest_id": "uuid-del-manifiesto",
  "generated_at": "2026-06-29T12:00:00Z",
  "tracks": {
    "uuid-track-1": { "version": 3, "updated_at": "2026-06-28T10:00:00Z" },
    "uuid-track-2": { "version": 1, "updated_at": "2026-06-25T08:00:00Z" }
  },
  "albums": {},
  "artists": {},
  "playlists": {}
}
```

---

#### `GET /sync/manifest/delta`

Obtiene un manifiesto diferencial desde una versión conocida. Útil para sincronización incremental.

**Parámetros de consulta:**

| Parámetro     | Tipo   | Obligatorio | Descripción                         |
|---------------|--------|-------------|--------------------------------------|
| `since`       | string | Sí          | Timestamp ISO 8601 de la última sincronización. |
| `manifest_id` | string | No          | ID del último manifiesto conocido.   |

**Respuesta `200 OK`:**

```json
{
  "manifest_id": "uuid-del-manifiesto",
  "generated_at": "2026-06-29T12:00:00Z",
  "since": "2026-06-28T12:00:00Z",
  "changes": {
    "tracks": {
      "added": ["uuid-track-nuevo"],
      "updated": ["uuid-track-modificado"],
      "removed": ["uuid-track-eliminado"]
    },
    "albums": {
      "added": [],
      "updated": [],
      "removed": []
    },
    "artists": {
      "added": [],
      "updated": [],
      "removed": []
    },
    "playlists": {
      "added": [],
      "updated": [],
      "removed": []
    }
  }
}
```

---

#### `POST /sync/state`

Sube el estado de sincronización de un dispositivo (tracks descargados, metadatos, etc.).

**Cuerpo de solicitud:**

```json
{
  "device_id": "uuid-del-dispositivo",
  "manifest_id": "uuid-del-manifiesto-aplicado",
  "synced_at": "2026-06-29T12:00:00Z",
  "downloaded_tracks": ["uuid-track-1", "uuid-track-2"],
  "storage_used_bytes": 5000000000
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "acknowledged_at": "2026-06-29T12:00:05Z"
}
```

---

### Reproducción

#### `GET /playback/state`

Obtiene el estado actual de reproducción del servidor.

**Respuesta `200 OK`:**

```json
{
  "state": "playing",
  "current_track": {
    "id": "uuid-del-track",
    "title": "Canción de Ejemplo",
    "artist": "Artista Ejemplo",
    "album": "Álbum Ejemplo",
    "duration_seconds": 245,
    "cover_id": "uuid-de-la-caratula"
  },
  "position_seconds": 78,
  "volume": 80,
  "device_id": "uuid-del-dispositivo-reproductor",
  "queue_id": "uuid-de-la-cola",
  "shuffle": false,
  "repeat": "off"
}
```

Posibles valores de `state`: `playing`, `paused`, `stopped`, `loading`.

Posibles valores de `repeat`: `off`, `one`, `all`.

---

#### `POST /playback/control`

Controla la reproducción: play, pause, stop, next, previous, seek.

**Cuerpo de solicitud:**

```json
{
  "action": "play",
  "position_seconds": 30
}
```

**Actions disponibles:**

| Acción     | Descripción                          | Requiere `position_seconds` |
|------------|--------------------------------------|-----------------------------|
| `play`     | Reanudar o iniciar reproducción.     | No                          |
| `pause`    | Pausar reproducción.                 | No                          |
| `stop`     | Detener reproducción.                | No                          |
| `next`     | Siguiente track.                     | No                          |
| `previous` | Track anterior.                      | No                          |
| `seek`     | Saltar a una posición específica.    | Sí                          |
| `volume`   | Cambiar volumen (0-100).             | No (usar `volume` en cuerpo)|

**Cuerpo alternativo para volumen:**

```json
{
  "action": "volume",
  "volume": 75
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "state": "playing",
  "position_seconds": 30
}
```

---

#### `POST /playback/session`

Crea o gestiona una sesión de reproducción. Permite iniciar una sesión en un dispositivo específico.

**Cuerpo de solicitud:**

```json
{
  "device_id": "uuid-del-dispositivo-reproductor",
  "action": "create",
  "queue_id": "uuid-de-la-cola"
}
```

**Actions disponibles:** `create`, `join`, `leave`, `transfer`.

**Respuesta `200 OK`:**

```json
{
  "session_id": "uuid-de-la-sesion",
  "device_id": "uuid-del-dispositivo-reproductor",
  "state": "active",
  "created_at": "2026-06-29T12:00:00Z"
}
```

---

### Cola

#### `GET /queue`

Obtiene la cola de reproducción actual.

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-de-la-cola",
  "current_index": 2,
  "items": [
    {
      "id": "uuid-item-cola",
      "track_id": "uuid-del-track",
      "title": "Canción de Ejemplo",
      "artist": "Artista Ejemplo",
      "duration_seconds": 245,
      "cover_id": "uuid-de-la-caratula",
      "added_by": "uuid-del-dispositivo",
      "added_at": "2026-06-29T11:55:00Z"
    }
  ],
  "total_duration_seconds": 4900,
  "shuffle": false,
  "repeat": "off"
}
```

---

#### `POST /queue/items`

Añade uno o más tracks a la cola de reproducción.

**Cuerpo de solicitud:**

```json
{
  "track_ids": ["uuid-track-1", "uuid-track-2"],
  "position": "next"
}
```

**Posiciones disponibles:** `next` (después del actual), `end` (al final), o un índice numérico.

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "items_added": 2,
  "queue_id": "uuid-de-la-cola"
}
```

---

#### `POST /queue/jump`

Salta a una posición específica dentro de la cola.

**Cuerpo de solicitud:**

```json
{
  "queue_item_id": "uuid-item-cola",
  "position_seconds": 0
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "current_index": 5,
  "current_track_id": "uuid-del-track"
}
```

---

#### `PUT /queue/reorder`

Reordena los elementos de la cola.

**Cuerpo de solicitud:**

```json
{
  "queue_item_ids": [
    "uuid-item-3",
    "uuid-item-1",
    "uuid-item-2"
  ]
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "queue_id": "uuid-de-la-cola"
}
```

---

#### `DELETE /queue/items/{id}`

Elimina un elemento específico de la cola.

**Respuesta `204 No Content`:** (sin cuerpo)

---

### Receptores

#### `GET /receivers`

Lista los receptores (dispositivos de reproducción) disponibles en la red.

**Respuesta `200 OK`:**

```json
{
  "data": [
    {
      "id": "uuid-del-receptor",
      "name": "Cocina Speaker",
      "device_type": "speaker",
      "state": "idle",
      "volume": 60,
      "is_active": true,
      "ip_address": "192.168.1.42",
      "last_seen": "2026-06-29T11:59:00Z",
      "capabilities": {
        "streaming_formats": ["flac", "mp3"],
        "max_bitrate": 320,
        "multiroom": true
      }
    }
  ],
  "total": 3
}
```

---

#### `GET /receivers/{id}`

Obtiene los detalles de un receptor específico.

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-del-receptor",
  "name": "Cocina Speaker",
  "device_type": "speaker",
  "state": "idle",
  "volume": 60,
  "is_active": true,
  "ip_address": "192.168.1.42",
  "port": 52051,
  "firmware_version": "1.2.3",
  "last_seen": "2026-06-29T11:59:00Z",
  "capabilities": {
    "streaming_formats": ["flac", "mp3"],
    "max_bitrate": 320,
    "multiroom": true
  }
}
```

---

#### `POST /receivers/{id}/session/start`

Inicia una sesión de reproducción en un receptor específico.

**Cuerpo de solicitud:**

```json
{
  "queue_id": "uuid-de-la-cola",
  "start_at_index": 0,
  "volume": 70
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "session_id": "uuid-de-la-sesion",
  "receiver_id": "uuid-del-receptor",
  "state": "playing"
}
```

---

#### `POST /receivers/{id}/session/stop`

Detiene la sesión de reproducción en un receptor.

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "receiver_id": "uuid-del-receptor",
  "previous_state": "playing"
}
```

---

#### `POST /receivers/{id}/volume`

Establece el volumen de un receptor.

**Cuerpo de solicitud:**

```json
{
  "volume": 75
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "receiver_id": "uuid-del-receptor",
  "volume": 75
}
```

---

### Salas

#### `GET /rooms`

Lista las salas (grupos de receptores) configuradas.

**Respuesta `200 OK`:**

```json
{
  "data": [
    {
      "id": "uuid-de-la-sala",
      "name": "Planta Baja",
      "description": "Salón y cocina",
      "members": [
        {
          "receiver_id": "uuid-receptor-1",
          "name": "Salón Speaker"
        },
        {
          "receiver_id": "uuid-receptor-2",
          "name": "Cocina Speaker"
        }
      ],
      "state": "idle",
      "created_at": "2026-04-01T10:00:00Z"
    }
  ],
  "total": 2
}
```

---

#### `POST /rooms`

Crea una nueva sala con uno o más receptores.

**Cuerpo de solicitud:**

```json
{
  "name": "Planta Alta",
  "description": "Dormitorios",
  "receiver_ids": ["uuid-receptor-3", "uuid-receptor-4"]
}
```

**Respuesta `201 Created`:**

```json
{
  "id": "uuid-de-la-sala",
  "name": "Planta Alta",
  "description": "Dormitorios",
  "members": [
    { "receiver_id": "uuid-receptor-3", "name": "Dormitorio Principal" },
    { "receiver_id": "uuid-receptor-4", "name": "Dormitorio Huéspedes" }
  ],
  "state": "idle",
  "created_at": "2026-06-29T12:00:00Z"
}
```

---

#### `GET /rooms/{id}`

Obtiene los detalles de una sala específica.

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-de-la-sala",
  "name": "Planta Baja",
  "description": "Salón y cocina",
  "members": [
    {
      "receiver_id": "uuid-receptor-1",
      "name": "Salón Speaker",
      "state": "playing",
      "volume": 60
    },
    {
      "receiver_id": "uuid-receptor-2",
      "name": "Cocina Speaker",
      "state": "playing",
      "volume": 50
    }
  ],
  "state": "playing",
  "created_at": "2026-04-01T10:00:00Z"
}
```

---

#### `PUT /rooms/{id}`

Actualiza la configuración de una sala.

**Cuerpo de solicitud:**

```json
{
  "name": "Planta Baja (Actualizado)",
  "description": "Nueva descripción",
  "receiver_ids": ["uuid-receptor-1", "uuid-receptor-2", "uuid-receptor-5"]
}
```

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-de-la-sala",
  "name": "Planta Baja (Actualizado)",
  "description": "Nueva descripción",
  "members": [
    { "receiver_id": "uuid-receptor-1", "name": "Salón Speaker" },
    { "receiver_id": "uuid-receptor-2", "name": "Cocina Speaker" },
    { "receiver_id": "uuid-receptor-5", "name": "Comedor Speaker" }
  ],
  "updated_at": "2026-06-29T12:10:00Z"
}
```

---

#### `DELETE /rooms/{id}`

Elimina una sala. Los receptores miembros no se ven afectados.

**Respuesta `204 No Content`:** (sin cuerpo)

---

#### `POST /rooms/{id}/play`

Inicia reproducción sincronizada en todos los miembros de la sala.

**Cuerpo de solicitud:**

```json
{
  "queue_id": "uuid-de-la-cola",
  "start_at_index": 0,
  "volume": 65
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "room_id": "uuid-de-la-sala",
  "session_id": "uuid-de-la-sesion-multiroom",
  "active_members": 3,
  "state": "playing"
}
```

---

### Eventos

#### `GET /events`

Conexión WebSocket para recibir eventos en tiempo real.

**URL:** `ws://{host}:{port}/api/v1/events`

**Autenticación:** Token vía query parameter: `?token=eyJ...`

**Eventos emitidos:**

| Evento                    | Descripción                                      |
|---------------------------|--------------------------------------------------|
| `playback.state_changed`  | Cambió el estado de reproducción.                |
| `playback.track_changed`  | Cambió el track actual.                          |
| `playback.progress`       | Actualización periódica de progreso (cada 1s).   |
| `queue.updated`           | La cola fue modificada.                          |
| `receiver.status_changed` | Un receptor cambió de estado.                    |
| `room.state_changed`      | Una sala cambió de estado.                       |
| `library.scan_started`    | Inició un escaneo de biblioteca.                 |
| `library.scan_completed`  | Finalizó un escaneo de biblioteca.               |
| `sync.manifest_updated`   | Nuevo manifiesto de sincronización disponible.   |
| `device.paired`           | Nuevo dispositivo emparejado.                    |
| `device.revoked`          | Dispositivo revocado.                            |

**Payload del evento:**

```json
{
  "event": "playback.track_changed",
  "data": {
    "track_id": "uuid-del-track",
    "title": "Canción de Ejemplo"
  },
  "timestamp": "2026-06-29T12:00:00Z"
}
```

---

### Receiver Lite (v1-lite)

Estos endpoints son implementados por los receptores ligeros (firmware nativo en speakers/amplificadores) y consumidos por el servidor. No requieren autenticación Bearer (usan un token interno de dispositivo).

#### `GET /receiver/info`

Obtiene la información de identidad del receptor.

**Respuesta `200 OK`:**

```json
{
  "device_id": "uuid-del-receptor",
  "device_name": "Cocina Speaker",
  "device_type": "speaker",
  "firmware_version": "1.2.3",
  "roles": ["receiver"],
  "capabilities": {
    "streaming_formats": ["flac", "mp3"],
    "max_bitrate": 320,
    "multiroom": true
  },
  "uptime_seconds": 604800
}
```

---

#### `POST /receiver/pair/start`

Inicia el emparejamiento desde el receptor hacia el servidor.

**Cuerpo de solicitud:**

```json
{
  "server_url": "http://192.168.1.100:52050",
  "device_name": "Cocina Speaker",
  "device_type": "speaker",
  "roles": ["receiver"],
  "capabilities": {
    "streaming_formats": ["flac", "mp3"],
    "max_bitrate": 320,
    "multiroom": true
  }
}
```

**Respuesta `200 OK`:**

```json
{
  "pairing_code": "ABCD-1234",
  "expires_in_seconds": 300
}
```

---

#### `POST /receiver/pair/confirm`

Confirma el emparejamiento del receptor.

**Cuerpo de solicitud:**

```json
{
  "device_id": "uuid-del-receptor",
  "pairing_code": "ABCD-1234"
}
```

**Respuesta `200 OK`:**

```json
{
  "token": "token-interno-del-receptor",
  "server_id": "uuid-del-servidor",
  "server_url": "http://192.168.1.100:52050"
}
```

---

#### `POST /receiver/heartbeat`

Mantiene viva la conexión del receptor con el servidor. Debe enviarse cada 30 segundos.

**Cuerpo de solicitud:**

```json
{
  "device_id": "uuid-del-receptor",
  "state": "idle",
  "volume": 60,
  "current_track_id": "uuid-del-track",
  "position_seconds": 78
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "server_time": "2026-06-29T12:00:00Z",
  "next_action": "none"
}
```

---

#### `POST /receiver/session/start`

El servidor solicita al receptor que inicie una sesión de reproducción.

**Cuerpo de solicitud:**

```json
{
  "session_id": "uuid-de-la-sesion",
  "stream_url": "http://192.168.1.100:52050/api/v1/stream/uuid-del-track",
  "token": "token-de-streaming",
  "volume": 70
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "session_id": "uuid-de-la-sesion",
  "state": "playing"
}
```

---

#### `POST /receiver/session/stop`

El servidor solicita al receptor que detenga la sesión.

**Cuerpo de solicitud:**

```json
{
  "session_id": "uuid-de-la-sesion"
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "previous_state": "playing"
}
```

---

#### `POST /receiver/volume`

El servidor establece el volumen del receptor.

**Cuerpo de solicitud:**

```json
{
  "volume": 75
}
```

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "volume": 75
}
```

---

#### `GET /receiver/firmware`

Consulta si hay una actualización de firmware disponible.

**Respuesta `200 OK`:**

```json
{
  "current_version": "1.2.3",
  "available_version": "1.3.0",
  "update_available": true,
  "download_url": "http://192.168.1.100:52050/firmware/v1.3.0.bin",
  "changelog": "Correcciones de seguridad y mejoras de rendimiento."
}
```
