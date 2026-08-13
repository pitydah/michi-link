# Michi Link API v1

- **API Version:** `v1` (permanent — only changes with a deliberate breaking change); receivers use the constrained profile `v1-lite`
- **Primary Transport:** HTTP/1.1 REST (HTTPS recommended) — all data operations
- **Real-time Transport:** WebSocket at `/api/v1/events` — real-time event notifications only
- **Discovery:** UDP multicast (`224.0.0.167:53318`) and/or mDNS (`_michi-link._tcp.local`)
- **Content-Type:** `application/json`
- **Authentication:** Bearer Token via `Authorization` header
- **Base URL:** `http://{host}:{port}/api/v1`

---

## Canonical Field Names

The following field names are **official** and MUST be used by all implementations. Legacy/aliased names (marked with →) are accepted during transition but MUST NOT appear in new code.

| Domain | Official Field | Type | Notes |
|--------|---------------|------|-------|
| Playback Control | `command` | string | `"action"` → legacy alias |
| Playback Control | `value` | int/string/bool | Parameter for `seek`, `set_volume`, `shuffle`, `repeat` |
| Playback State | `position_ms` | int | Milliseconds. `position_seconds` → legacy alias |
| Playback State | `state` | string | `"playing"`, `"paused"`, `"stopped"`, `"loading"` |
| Playback State | `volume` | int | 0–100 |
| Playback State | `repeat` | string | `"off"`, `"one"`, `"all"` |
| Sync Delta | `cursor` | string | Opaque cursor token. `since`, `manifest_id` → legacy aliases |
| Sync Manifest | `cursor` | string | Next cursor for delta requests |
| Track | `duration_ms` | int | Milliseconds. `duration_seconds` → legacy alias |
| Volume | `volume` | int | 0–100 inclusive |
| API Version | `api_version` | string | Enum: `"v1"` (servidores completos) o `"v1-lite"` (receptores). Nunca un semver. El antiguo campo de versión de enlace fue retirado del contrato: no usarlo. |

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

Los tokens se obtienen mediante el flujo de pairing (`/pair/start`, `/pair/confirm`) y se renuevan vía `/token/refresh` cuando el servidor lo soporta.

Cada servidor declara su estrategia de autenticación en `GET /server/info` → `auth.strategy`. Ver [AUTH_PROFILES.md](AUTH_PROFILES.md) para la documentación completa de las estrategias:

| Estrategia | Proyecto | token_refresh | Código |
|------------|----------|---------------|--------|
| `PLAYER_PASSWORD` | Michi Music Player | No | Contraseña configurada por el usuario |
| `SERVER_CODE` | Michi Micro Server | Sí | Código temporal en pantalla |
| `RECEIVER_BUTTON` | Michi Music Stream | No | Botón físico |
| `LEGACY` | Transición | Variable | Fallback para clientes antiguos |

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
| `CONFLICT`                 | Conflicto de estado, replay o sesión duplicada. |
| `RATE_LIMITED`             | Demasiadas solicitudes o intentos. Incluye header `Retry-After`. |
| `INTERNAL_ERROR`           | Error interno del servidor.            |
| `NOT_IMPLEMENTED`          | Endpoint o funcionalidad no implementada. |
| `PAIRING_NOT_FOUND`        | Sesión de pairing inexistente.         |
| `PAIRING_EXPIRED`          | La sesión de pairing expiró.           |
| `PAIRING_ATTEMPTS_EXCEEDED`| Se superó el máximo de intentos.       |
| `PAIRING_KEY_MISMATCH`     | La clave del cliente no coincide con la sesión. |
| `PAIRING_ALREADY_CONSUMED` | La sesión de pairing ya fue consumida. |
| `PAIRING_PIN_MISMATCH`     | El PIN no corresponde a la sesión.     |
| `SIGNATURE_INVALID`        | Firma inválida.                        |
| `REPLAY_DETECTED`          | Replay detectado.                      |
| `IDENTITY_CORRUPTED`       | Identidad persistida corrupta (exige factory reset). |
| `IDEMPOTENCY_KEY_REUSE`    | El `Idempotency-Key` ya fue usado con otro método/ruta. |
| `TRACK_NOT_FOUND`          | Track no encontrado en la biblioteca.  |
| `IMPORT_SESSION_EXPIRED`   | La sesión de import expiró.            |

Los 20 códigos canónicos están definidos en `schemas/error.schema.json`. Los clientes solo ramifican por `code` y status HTTP; `message` es para humanos.

**Ejemplos de errores:**

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "El cuerpo de la solicitud contiene campos inválidos.",
    "details": { "field": "volume", "reason": "debe ser un entero entre 0 y 100" }
  }
}
```

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Token ausente o inválido.",
    "details": {}
  }
}
```

```json
{
  "error": {
    "code": "FORBIDDEN",
    "message": "El token no tiene el permiso 'stream.read'.",
    "details": { "required_permission": "stream.read" }
  }
}
```

```json
{
  "error": {
    "code": "NOT_IMPLEMENTED",
    "message": "El endpoint /api/v1/receivers no está implementado en este servidor.",
    "details": {}
  }
}
```

```json
{
  "error": {
    "code": "TRACK_NOT_FOUND",
    "message": "Track no encontrado.",
    "details": { "track_id": "uuid-inexistente" }
  }
}
```

```json
{
  "error": {
    "code": "RANGE_NOT_SATISFIABLE",
    "message": "Rango de bytes solicitado no válido.",
    "details": { "content_length": 35000000, "requested_range": "bytes=99999999-" }
  }
}
```

---

## Request Validation

Todo servidor DEBE validar los requests entrantes contra los schemas definidos antes de procesarlos. Los errores de validación se reportan con:

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "El cuerpo de la solicitud contiene campos inválidos.",
    "details": {
      "field": "volume",
      "reason": "must be between 0 and 100",
      "value": 150
    }
  }
}
```

### Reglas de validación

| Tipo | Regla | Ejemplo |
|------|-------|---------|
| String | `minLength: 1` | Campos obligatorios no vacíos |
| String | `maxLength` | Límite superior para evitar abusos |
| String | `pattern` | Formato específico (UUID, hash, base64) |
| Integer | `minimum`, `maximum` | Rangos numéricos (volumen 0-100, año 1900-2100) |
| Array | `minItems`, `maxItems` | Bulk operations limitadas a 500 items |
| Array | `uniqueItems` | Sin duplicados en colecciones |

---

## Idempotency

Los endpoints POST, PUT y DELETE pueden aceptar un header `Idempotency-Key` para garantizar que requests repetidos tengan el mismo efecto que uno solo.

### Header

```
Idempotency-Key: <string>
```

### Comportamiento

1. El servidor almacena la combinación `(key, método, ruta)` y la respuesta original durante **1 hora**.
2. Si el mismo `Idempotency-Key` llega dentro de la ventana, el servidor devuelve la respuesta almacenada sin procesar el request.
3. Si la key ya fue usada con **diferente** método o ruta, el servidor responde con `422 IDEMPOTENCY_KEY_REUSE`.

### Endpoints que soportan idempotency

| Endpoint | Método |
|----------|--------|
| `/api/v1/import/session` | POST |
| `/api/v1/import/upload/{session_id}` | POST |
| `/api/v1/import/commit/{session_id}` | POST |
| `/api/v1/queue/items` | POST |
| `/api/v1/playlists` | POST |
| `/api/v1/playlists/{id}/tracks` | PUT |
| `/api/v1/tracks/bulk` | POST |
| `/api/v1/queue/items/bulk` | POST |

---

## Compression

El servidor DEBE soportar compresión gzip para respuestas JSON.

### Request

```
Accept-Encoding: gzip
```

### Response

```
Content-Encoding: gzip
```

Las respuestas JSON con tamaño mayor a 1KB se comprimen automáticamente. Las respuestas de streaming (`/stream/{id}`, `/download/{id}`) NO se comprimen.

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

Endpoints públicos (sin autenticación). Obtiene la identidad, roles activos y capacidades del servidor.

**Campos oficiales:**

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `service` | string | Identificador del servicio: `michi-music-player`, `michi-micro-server`, `michi-mobile`, `michi-stream-standard`, `michi-stream-hifi` |
| `name` | string | Nombre legible del servidor (configurable por el usuario) |
| `server_id` | string | UUID único del servidor |
| `version` | string | Versión de la aplicación |
| `api_version` | string | Versión del contrato API: enum `"v1"` (servidores completos) o `"v1-lite"` (receptores). Nunca cambia sin un breaking change deliberado. |
| `roles` | string[] | Roles activos del servidor (lista oficial en ARCHITECTURE.md) |
| `features` | object | Indicadores booleanos de capacidades. |
| `auth` | object | Información de autenticación (ver AUTH_PROFILES.md) |
| `auth.required` | boolean | Si la autenticación es obligatoria (`true` para todos los servidores v1) |
| `auth.strategy` | string | Estrategia: `PLAYER_PASSWORD`, `SERVER_CODE`, `RECEIVER_BUTTON`, `LEGACY` |
| `auth.token_refresh` | boolean | Si el servidor soporta `/token/refresh` |

**Campos eliminados** (no usar): `server_name`, `server_version`, `device_id`, `capabilities`, roles genéricos como `core`, `sync_leader`, `library_service`.

**Respuesta `200 OK` (Michi Micro Server):**

```json
{
  "service": "michi-micro-server",
  "name": "Michi Micro Server",
  "server_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "version": "0.1.0",
  "api_version": "v1",
  "roles": ["home_server", "library_server", "stream_server"],
  "features": {
    "library": true,
    "search": true,
    "streaming": true,
    "sync_manifest": true,
    "playback": true,
    "queue": true,
    "artwork": true,
    "events": true
  },
  "auth": { "required": true, "strategy": "SERVER_CODE", "token_refresh": true }
}
```

**Respuesta `200 OK` (Michi Music Player):**

```json
{
  "service": "michi-music-player",
  "name": "Michi Music Player",
  "server_id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
  "version": "0.1.0",
  "api_version": "v1",
  "roles": ["desktop_player", "library_master", "sync_host", "sync_source", "stream_server"],
  "features": {
    "library": true,
    "search": true,
    "streaming": true,
    "sync_manifest": true,
    "playback": true,
    "queue": true,
    "artwork": true,
    "events": false
  },
  "auth": { "required": true, "strategy": "PLAYER_PASSWORD", "token_refresh": false }
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

Flujo único canónico de emparejamiento: **sesión + PIN + challenge Ed25519**. El cliente prueba posesión de su clave firmando un nonce en `/pair/start`; el servidor abre una sesión y muestra un **PIN de 6 dígitos en su propia pantalla** (nunca viaja por la red); el cliente completa con `/pair/confirm`. Schemas: `schemas/pair-start.schema.json`, `pair-start-response`, `pair-confirm`, `pair-confirm-response`.

#### `POST /pair/start`

Inicia el emparejamiento. El cuerpo lleva la identidad del cliente (base64url estricto: `michi_id`/`public_key` 43 chars, `challenge_nonce` ≥ 22 chars, `challenge_signature` 86 chars) y el challenge Ed25519 (firma sobre los **bytes crudos** del nonce), que prueba posesión de la clave. El PIN se muestra en el servidor y **nunca** se devuelve al cliente.

**Cuerpo de solicitud:**

```json
{
  "device_name": "Michi Micro Server",
  "device_type": "server",
  "roles": ["music_server"],
  "auth_strategy": "RECEIVER_BUTTON",
  "michi_id": "97ryPKOLZ-JgVKQFc2ZuuSk0alWzxagdNILuDW26jEc",
  "public_key": "fDBBmExOH6h74KpGq2ckfDNN0Mzi7oMN4g_V2IKAR8Y",
  "challenge_nonce": "VFfZjzw8JeAM7-RFiTSrMA",
  "challenge_signature": "DTlMt9BYH_TnYgKAeGd8zTpza-w5b8BDm9AyIoAW2p0clD7JrzwN9cwPY5y48K14x_0z2TPq7-LTXdNTqmhr-w"
}
```

El servidor valida la firma sobre los bytes decodificados de `challenge_nonce` y que `michi_id` corresponde a `public_key`; un fallo responde `400 INVALID_REQUEST` y no crea sesión.

**Respuesta `201 Created`:**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440001",
  "expires_at": "2026-08-12T20:02:00Z",
  "attempts_remaining": 5,
  "server_michi_id": "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
  "server_public_key": "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ"
}
```

En receptores `RECEIVER_BUTTON`, el pairing solo se acepta dentro de la ventana física de 120 s; fuera de ella responde **`403 Forbidden`** (`FORBIDDEN`). **`429 Rate Limited`** si se supera un límite del registry (1024 globales, 8 por origen, 4 por identidad, 20 starts/min por origen):

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Demasiadas sesiones de pairing activas.",
    "details": {}
  }
}
```

---

#### `POST /pair/confirm`

Confirma el emparejamiento con el PIN obtenido en la pantalla del servidor.

**Cuerpo de solicitud:**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440001",
  "pin": "042731",
  "michi_id": "97ryPKOLZ-JgVKQFc2ZuuSk0alWzxagdNILuDW26jEc",
  "public_key": "fDBBmExOH6h74KpGq2ckfDNN0Mzi7oMN4g_V2IKAR8Y"
}
```

**Respuesta `200 OK`:**

```json
{
  "token": "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE",
  "expires_in": 0,
  "device_id": "550e8400-e29b-41d4-a716-446655440002",
  "server_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

`token` es un **bearer token opaco** (no JWT), generado por el servidor, devuelto una sola vez y persistido únicamente como digest. `refresh_token` es opcional (presente solo si el servidor soporta `/token/refresh`, ej. Micro Server). En receptores `expires_in: 0` significa "sin expiración automática; válido hasta revocación o factory reset". La sesión se consume tras éxito; una segunda confirmación responde `409 CONFLICT` (`PAIRING_ALREADY_CONSUMED`).

**Respuesta `401 Unauthorized`** (PIN incorrecto):

```json
{
  "error": {
    "code": "PAIRING_PIN_MISMATCH",
    "message": "El PIN no corresponde a esta sesión de pairing.",
    "details": {}
  }
}
```

Errores de pairing canónicos: `PAIRING_NOT_FOUND`, `PAIRING_EXPIRED`, `PAIRING_ATTEMPTS_EXCEEDED`, `PAIRING_ALREADY_CONSUMED`, `PAIRING_KEY_MISMATCH`, `PAIRING_PIN_MISMATCH` y `RATE_LIMITED` (ver `schemas/error.schema.json`, 20 códigos).

---

### Tokens

#### `POST /token/refresh`

Renueva el token de acceso usando el refresh token.

**Cuerpo de solicitud:**

```json
{
  "refresh_token": "tok_michi_refresh_c2b9..."
}
```

**Respuesta `200 OK`:**

```json
{
  "token": "tok_michi_opaco_7f3a...",
  "refresh_token": "tok_michi_refresh_c2b9...",
  "expires_in": 3600
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
    "code": "RANGE_NOT_SATISFIABLE",
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

### Bulk Operations

#### `POST /tracks/bulk`

Crea múltiples tracks en una sola operación. Límite: 500 tracks por request.

**Auth:** Bearer token con permiso `library.write`.

**Cuerpo de solicitud:**

```json
{
  "tracks": [
    {
      "id": "track_001",
      "title": "Neon Lights",
      "artist": "Luna Swift",
      "album": "Imaginary Cities",
      "duration_ms": 245000,
      "format": "flac"
    }
  ]
}
```

**Respuesta `201 Created`:**

```json
{
  "created": 2,
  "failed": 0,
  "tracks": [
    { "id": "track_001", "status": "created" },
    { "id": "track_002", "status": "created" }
  ]
}
```

**Respuesta `400 Bad Request`** si algún track no pasa validación:

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Error de validación en track index 0.",
    "details": { "index": 0, "field": "duration_ms", "reason": "must be a non-negative integer" }
  }
}
```

#### `POST /queue/items/bulk`

Agrega múltiples tracks a la cola de reproducción en una sola operación. Límite: 500 tracks.

**Auth:** Bearer token con permiso `queue.write`.

**Cuerpo de solicitud:**

```json
{
  "track_ids": [
    "uuid-track-1",
    "uuid-track-2",
    "uuid-track-3"
  ],
  "position": "end"
}
```

**Respuesta `200 OK`:**

```json
{
  "items_added": 3,
  "queue_id": "uuid-de-la-cola"
}
```

#### `POST /playlists/{id}/tracks/bulk`

Agrega múltiples tracks a una playlist existente. Límite: 500 tracks.

**Auth:** Bearer token con permiso `playlist.write`.

**Cuerpo de solicitud:**

```json
{
  "track_ids": ["uuid-track-1", "uuid-track-2"]
}
```

**Respuesta `200 OK`:**

```json
{
  "id": "uuid-de-la-playlist",
  "tracks_added": 2,
  "track_count": 27
}
```

---

### Sincronización

#### `GET /sync/manifest`

Obtiene el manifiesto completo de sincronización. Contiene todos los elementos de la biblioteca con sus versiones.

**Respuesta `200 OK`:**

```json
{
  "cursor": "initial_cursor_string",
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

Obtiene un manifiesto diferencial desde un cursor conocido. Útil para sincronización incremental.

**Parámetros de consulta:**

| Parámetro | Tipo | Obligatorio | Descripción |
|-----------|------|-------------|-------------|
| `cursor` | string | Sí | Cursor opaco de la última sincronización (devuelto por `/sync/manifest` o por la respuesta delta anterior). |
| `device_id` | string | Sí | ID del dispositivo solicitante. |

> **Nota de compatibilidad:** Durante la transición se aceptan también `since` (timestamp ISO 8601) y `manifest_id` (ID de manifiesto). El campo oficial es `cursor`. Las nuevas implementaciones DEBEN usar `cursor`.

**Respuesta `200 OK`:**

```json
{
  "cursor": "cursor_delta_001_a1b2c3d4",
  "added": [
    { "type": "track", "id": "track_nuevo_001", "data": { "title": "Nueva Canción", "artist": "Artista" } }
  ],
  "updated": [
    { "type": "album", "id": "album_mod_001", "data": { "title": "Título Actualizado" } }
  ],
  "deleted": [
    { "type": "track", "id": "track_eliminado_001" }
  ],
  "playlists_updated": [
    { "playlist_id": "playlist_001", "added": ["track_004"], "removed": ["track_002"] }
  ]
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
    "duration_ms": 245000,
    "cover_id": "uuid-de-la-caratula"
  },
  "position_ms": 78000,
  "volume": 80,
  "device_id": "uuid-del-dispositivo-reproductor",
  "queue_id": "uuid-de-la-cola",
  "shuffle": false,
  "repeat": "off"
}
```

Posibles valores de `state`: `playing`, `paused`, `stopped`, `loading`.

Posibles valores de `repeat`: `off`, `one`, `all`.

Volumen oficial: entero 0–100.

---

#### `POST /playback/control`

Controla la reproducción: play, pause, stop, next, previous, seek, volume, shuffle, repeat.

**Cuerpo de solicitud (oficial):**

```json
{
  "command": "seek",
  "value": 30000
}
```

**Comandos oficiales:**

| Comando        | Descripción                              | `value` esperado        |
|----------------|------------------------------------------|--------------------------|
| `play`         | Reanudar o iniciar reproducción.         | `null`                   |
| `pause`        | Pausar reproducción.                     | `null`                   |
| `toggle`       | Alternar play/pausa.                     | `null`                   |
| `stop`         | Detener reproducción.                    | `null`                   |
| `next`         | Siguiente track.                         | `null`                   |
| `previous`     | Track anterior.                          | `null`                   |
| `seek`         | Saltar a posición específica.            | `int` (position_ms)      |
| `set_volume`   | Establecer volumen (0–100).              | `int` (0–100)            |
| `mute`         | Silenciar.                               | `null`                   |
| `unmute`       | Reactivar sonido.                        | `null`                   |
| `shuffle`      | Activar/desactivar shuffle.              | `boolean`                |
| `repeat`       | Establecer modo de repetición.           | `"off"`, `"one"`, `"all"`|

> **Nota de compatibilidad:** El campo `"action"` se acepta como alias legacy durante la transición, pero el campo oficial es `"command"`. Las nuevas implementaciones DEBEN usar `"command"`.

**Respuesta `200 OK`:**

```json
{
  "success": true,
  "state": "playing",
  "position_ms": 30000
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

Gestión server-side de receptores (consumida por Micro Server). El control efectivo del receptor (sesión, volumen, heartbeat) se hace a través de los endpoints [Receiver Lite (v1-lite)](#receiver-lite-v1-lite), no aquí.

#### `GET /receivers`

Lista los receptores descubiertos y registrados en la red.

**Respuesta `200 OK`:**

```json
{
  "receivers": [
    {
      "device_id": "uuid-del-receptor",
      "name": "Cocina Speaker",
      "device_type": "receiver",
      "roles": ["audio_receiver"],
      "permissions": ["receiver.status"],
      "last_seen": "2026-08-12T11:59:00Z",
      "created_at": "2026-08-12T10:00:00Z"
    }
  ]
}
```

---

#### `POST /receivers/discover`

Dispara un descubrimiento de receptores en la red.

**Parámetro de consulta:** `timeout` (int, 1–30, defecto 5) — duración del descubrimiento en segundos.

**Respuesta `200 OK`:**

```json
{
  "discovered": [
    {
      "device_id": "uuid-del-receptor",
      "name": "Cocina Speaker",
      "device_type": "receiver",
      "roles": ["audio_receiver"]
    }
  ]
}
```

---

#### `GET /receivers/{receiverId}`

Obtiene los detalles de un receptor específico.

**Respuesta `200 OK`:**

```json
{
  "device_id": "uuid-del-receptor",
  "name": "Cocina Speaker",
  "device_type": "receiver",
  "roles": ["audio_receiver"],
  "permissions": ["receiver.status", "receiver.session", "receiver.volume", "receiver.now_playing"],
  "last_seen": "2026-08-12T11:59:00Z",
  "created_at": "2026-08-12T10:00:00Z"
}
```

**Respuesta `404 Not Found`** si el receptor no existe.

> No existen `POST /receivers/{id}/session/start`, `POST /receivers/{id}/session/stop` ni `POST /receivers/{id}/volume`: fueron retirados. El ciclo de sesión del receptor se maneja exclusivamente con `POST/GET/PATCH/DELETE /receiver-lite/session`.

---

### Salas

> El multiroom está fuera del alcance de esta convergencia: los receptores v1-lite no implementan salas ni sincronización de reloj. Estos endpoints quedan documentados como superficie server-side para trabajo futuro.

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
  "session_id": "uuid-de-la-sesion-sala",
  "active_members": 3,
  "state": "playing"
}
```

---

### Eventos

#### `GET /events`

Conexión WebSocket para recibir eventos en tiempo real.

**URL:** `ws://{host}:{port}/api/v1/events`

**Autenticación:** Token opaco vía query parameter: `?token=tok_michi_opaco_7f3a...`

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

Perfil canónico para receptores físicos (Michi Music Stream). Congelado por ADR-0001 y documentado en detalle en [RECEIVERS_V1_LITE.md](RECEIVERS_V1_LITE.md); el contrato ejecutable vive en el bundle `contracts/receiver-v1-lite/` (OpenAPI + schemas + vectores). El receptor declara `api_version: "v1-lite"`, `service: "michi-stream-standard" | "michi-stream-hifi"`, `roles: ["audio_receiver"]` y `auth: { "required": true, "strategy": "RECEIVER_BUTTON", "token_refresh": false }`.

Un receptor v1-lite **no declara**: library, playlists, search, sync, storage, reproducción autónoma, transcoding, rooms ni token refresh.

#### Tabla de rutas

Todas las rutas empiezan en `/api/v1`. Cuerpos JSON en `snake_case`, UTF-8, `Content-Type: application/json`. Salvo `204`, todo error usa el schema canónico `Error`.

| Método | Ruta | Éxito | Auth | Feature |
|--------|------|------:|------|---------|
| `GET` | `/api/v1/server/info` | `200` | no | siempre |
| `POST` | `/api/v1/pair/start` | `201` | no; ventana física abierta | siempre |
| `GET` | `/api/v1/pair/status` | `200` | no; `session_id` query | siempre |
| `POST` | `/api/v1/pair/confirm` | `200` | no; sesión de pairing | siempre |
| `POST` | `/api/v1/receiver-lite/session` | `201` | Bearer | `session` |
| `GET` | `/api/v1/receiver-lite/session` | `200` | Bearer | `session` |
| `PATCH` | `/api/v1/receiver-lite/session` | `200` | Bearer + sesión | `session` |
| `DELETE` | `/api/v1/receiver-lite/session` | `204` | Bearer + sesión | `session` |
| `POST` | `/api/v1/receiver-lite/heartbeat` | `200` | Bearer + sesión | `heartbeat` |
| `PUT` | `/api/v1/receiver-lite/now-playing` | `204` | Bearer + sesión | `now_playing` opcional |
| `GET` | `/api/v1/receiver-lite/diagnostics` | `200` | Bearer | `diagnostics` opcional |
| `GET` | `/api/v1/receiver-lite/firmware` | `200` | Bearer | `ota` opcional |
| `POST` | `/api/v1/receiver-lite/firmware` | `202` | Bearer + permiso OTA | `ota` opcional |

No existen `/receiver/info`, `/receiver/session/start`, `/receiver/session/stop`, `/receiver/pair/*`, `/receiver-lite/volume`, `/receiver-lite/info` ni `/receiver-lite/config`.

#### `GET /server/info` (receptor)

Respuesta exacta para Standard; Hi-Fi solo cambia `service` por `michi-stream-hifi` hasta que exista certificación adicional:

```json
{
  "service": "michi-stream-standard",
  "name": "Michi Stream Cocina",
  "server_id": "550e8400-e29b-41d4-a716-446655440000",
  "version": "0.3.0",
  "api_version": "v1-lite",
  "roles": ["audio_receiver"],
  "identity_scheme": "ed25519-blake3-v1",
  "michi_id": "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
  "public_key": "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ",
  "auth": { "required": true, "strategy": "RECEIVER_BUTTON", "token_refresh": false },
  "features": { "session": true, "heartbeat": true, "volume": true, "now_playing": true, "diagnostics": true, "ota": true },
  "audio": {
    "transports": ["rtp_udp"], "codecs": ["pcm_s16le"], "sample_rates": [48000],
    "bit_depths": [16], "channels": [2], "packet_ms": [10], "payload_types": [97],
    "buffer_ms_min": 50, "buffer_ms_max": 500
  }
}
```

- `server_id` es UUID v4 estable, generado una vez y persistido en NVS.
- `michi_id` deriva de `public_key`; no es igual a `server_id`.
- Los tres campos de identidad son obligatorios para `michi-stream-*`; `roles` contiene exactamente `audio_receiver`; `version` es la versión de firmware.
- Una feature vale `true` solo si su handler está registrado y tiene prueba positiva.
- `audio` declara capacidad reproducible, no la capacidad teórica del DAC.

#### Pairing (`RECEIVER_BUTTON`)

- Una **pulsación física** abre una ventana de 120 s. Reiniciar la cierra; abrir de nuevo reemplaza la ventana previa. Fuera de la ventana, `POST /pair/start` responde `403 FORBIDDEN`.
- `POST /pair/start` valida el challenge Ed25519 (firma sobre los bytes crudos de `challenge_nonce`) y que `michi_id` corresponde a `public_key`; un fallo responde `400 INVALID_REQUEST` y no crea sesión. Crea un PIN de 6 dígitos aleatorio, lo muestra localmente y **no** lo devuelve por HTTP. Bajo el modelo de confianza LAN, el cliente envía el PIN únicamente en `POST /pair/confirm`.

```json
{
  "device_name": "Michi Micro Server",
  "device_type": "server",
  "roles": ["music_server"],
  "auth_strategy": "RECEIVER_BUTTON",
  "michi_id": "97ryPKOLZ-JgVKQFc2ZuuSk0alWzxagdNILuDW26jEc",
  "public_key": "fDBBmExOH6h74KpGq2ckfDNN0Mzi7oMN4g_V2IKAR8Y",
  "challenge_nonce": "VFfZjzw8JeAM7-RFiTSrMA",
  "challenge_signature": "DTlMt9BYH_TnYgKAeGd8zTpza-w5b8BDm9AyIoAW2p0clD7JrzwN9cwPY5y48K14x_0z2TPq7-LTXdNTqmhr-w"
}
```

- `GET /pair/status?session_id=<uuid>` responde `status` `pending` / `confirmed` / `expired` / `locked`; sesión inexistente: `404 NOT_FOUND`. Máximo cinco intentos fallidos de PIN; después `429 RATE_LIMITED` y la sesión queda consumida.
- `POST /pair/confirm` verifica identidad/clave exactas de `/pair/start` y el PIN. El **token lo genera el receptor**: 32 bytes CSPRNG, base64url sin padding, devuelto una sola vez; el receptor persiste únicamente SHA-256 del token. `expires_in: 0` = sin expiración automática, válido hasta revocación o factory reset. La sesión se consume tras éxito; un segundo confirm responde `409 CONFLICT`.

```json
{
  "token": "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE",
  "expires_in": 0,
  "device_id": "550e8400-e29b-41d4-a716-446655440002",
  "server_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

Permisos mínimos emitidos tras pairing: `receiver.status`, `receiver.session`, `receiver.volume`, `receiver.now_playing`. `receiver.ota` no se concede por defecto.

#### Autenticación HTTP

- Controlador: `Authorization: Bearer <pairing_token>`.
- Mutaciones de una sesión activa añaden `X-Michi-Session: <session_token>`.
- `session_token` es distinto del pairing token: 32 bytes aleatorios base64url sin padding, solo en RAM.
- `GET /server/info` y pairing no usan Bearer. `GET /receiver-lite/session` requiere Bearer pero no `X-Michi-Session`. `PATCH`, `DELETE`, heartbeat y now-playing exigen ambos.
- Nunca aceptar tokens en query string o cuerpo JSON. Comparar digests en tiempo constante.

#### `POST /receiver-lite/session`

Crea la única sesión de audio. Todos los campos son obligatorios; `additionalProperties: false`:

```json
{
  "transport": "rtp_udp",
  "codec": "pcm_s16le",
  "sample_rate": 48000,
  "bit_depth": 16,
  "channels": 2,
  "packet_ms": 10,
  "buffer_ms": 120,
  "payload_type": 97,
  "ssrc": 305419896,
  "volume": 70
}
```

| Campo | Valor/rango admitido |
|-------|----------------------|
| `transport` | exactamente `rtp_udp` |
| `codec` | exactamente `pcm_s16le` |
| `sample_rate` | exactamente `48000` |
| `bit_depth` | exactamente `16` |
| `channels` | exactamente `2` |
| `packet_ms` | exactamente `10` |
| `buffer_ms` | entero `50..500` |
| `payload_type` | exactamente `97` |
| `ssrc` | entero sin signo `1..4294967295` |
| `volume` | entero `0..100` |

- No redondear ni corregir valores inválidos: `400 INVALID_REQUEST` con `details.field`.
- Si ya hay sesión activa: `409 CONFLICT`.
- El receptor elige un puerto UDP libre en `49152..65535`. La IP fuente RTP se fija a la IP TCP del request HTTP; no se acepta `source_ip` en JSON.
- No iniciar audio hasta reservar socket, buffer y motor con éxito; ante fallo parcial, rollback completo a `idle`.

**Respuesta `201 Created`:**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "session_token": "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
  "lease_seconds": 30,
  "effective": {
    "transport": "rtp_udp", "codec": "pcm_s16le", "sample_rate": 48000,
    "bit_depth": 16, "channels": 2, "packet_ms": 10, "buffer_ms": 120,
    "payload_type": 97, "ssrc": 305419896, "stream_port": 55300, "volume": 70
  }
}
```

**RTP aceptado:** RTP v2 sin CSRC/extension/padding; PT `97`; SSRC exactamente el negociado (sin "first packet wins"); IPv4 origen exactamente la inferida al crear la sesión; PCM little-endian interleaved L/R 16-bit 48 kHz; con 10 ms cada paquete lleva 480 frames, 960 samples, 1920 bytes de payload. Paquetes con fuente/PT/SSRC/tamaño incorrecto se rechazan y contabilizan. La secuencia puede envolver; pérdida y reordenamiento se detectan sin cerrar la sesión.

#### `GET /receiver-lite/session`

**Respuesta `200 OK`** si hay sesión (`state`: `starting`, `playing`, `paused` o `stopping`):

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "state": "playing",
  "lease_remaining_ms": 24500,
  "volume": 70,
  "paused": false,
  "stream_port": 55300,
  "ssrc": 305419896,
  "packets_received": 1250,
  "packets_rejected": 0,
  "packets_lost": 0,
  "underruns": 0
}
```

Nunca devolver `session_token`. Sin sesión: `404 NOT_FOUND`.

#### `PATCH /receiver-lite/session`

Solo se admiten `volume` `0..100` y `paused` booleano, con al menos una propiedad:

```json
{
  "volume": 55,
  "paused": true
}
```

**Respuesta `200 OK`:** mismo cuerpo de estado de `GET` después de aplicar el cambio. No existe `/volume` separado.

#### `DELETE /receiver-lite/session`

Sin cuerpo. Éxito idempotente para la sesión autenticada: `204`. Token de sesión incorrecto: `401 UNAUTHORIZED`. Sin sesión: `404 NOT_FOUND`. Cierre: dejar de aceptar RTP, silenciar, detener motor, liberar buffers/socket y borrar el token en RAM.

#### `POST /receiver-lite/heartbeat`

Cada 10 segundos:

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "sequence": 7,
  "sent_at_ms": 1786564800000
}
```

- `sequence`: entero sin signo, estrictamente creciente dentro de la sesión. Repetido o anterior: `409 CONFLICT`, no renueva.
- `sent_at_ms`: Unix epoch en milisegundos; informativo, no se usa para el timeout local.
- Un heartbeat válido renueva el lease a 30 segundos.

**Respuesta `200 OK`:**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440003",
  "status": "alive",
  "lease_seconds": 30,
  "receiver_uptime_ms": 918273
}
```

El watchdog usa reloj monotónico. Al vencer 30 s: el mismo cierre seguro que `DELETE`, incrementar `lease_expirations` y volver a `idle` (aunque siga llegando RTP).

#### Extensiones opcionales

`PUT /receiver-lite/now-playing`, `GET /receiver-lite/diagnostics` y `GET/POST /receiver-lite/firmware` son extensiones opcionales anunciadas por feature flags. Sus shapes no congelados se definen cuando cada extensión se certifique. `POST /receiver-lite/firmware` exige permiso `receiver.ota` (no otorgado por defecto).

#### Mapa de errores del receptor

| Condición | HTTP | `code` |
|-----------|-----:|--------|
| JSON/campo/valor inválido | `400` | `INVALID_REQUEST` |
| Bearer o token de sesión ausente/inválido | `401` | `UNAUTHORIZED` |
| Token válido sin permiso o ventana física cerrada | `403` | `FORBIDDEN` |
| Sesión/recurso inexistente | `404` | `NOT_FOUND` |
| Estado incompatible, replay o sesión duplicada | `409` | `CONFLICT` |
| Exceso de intentos/requests | `429` | `RATE_LIMITED` |
| Feature no implementada | `501` | `NOT_IMPLEMENTED` |
| Error inesperado | `500` | `INTERNAL_ERROR` |

---

## Post-Beta Features

Las siguientes características están documentadas para futura implementación pero no son necesarias para beta.

### Maintenance Mode

Un servidor puede indicar que está en mantenimiento respondiendo con `503 Service Unavailable`:

```json
{
  "status": "maintenance",
  "message": "Server is undergoing maintenance. Expected completion in 15 minutes.",
  "estimated_downtime_seconds": 900
}
```

Header: `Retry-After: 900`

### CORS

Para clientes web (dashboards, Home Assistant), el servidor DEBE implementar CORS:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization, Idempotency-Key
```

### Content-Type Negotiation

El servidor PUEDE responder `406 Not Acceptable` si el header `Accept` solicita un tipo de contenido no soportado (ej: `application/xml`). El único Content-Type soportado es `application/json`.

### WebSocket Event Retry

Si un cliente WebSocket se desconecta, debe reconectar con backoff exponencial:

1. Esperar 1 segundo.
2. Si falla, esperar 2 segundos.
3. Si falla, esperar 4, 8, 16... hasta un máximo de 30 segundos.
4. Al reconectar, el servidor reenvía el último evento de estado conocido.