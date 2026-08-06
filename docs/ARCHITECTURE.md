# Arquitectura de Michi Link Ecosystem

## Diagrama de alto nivel

```
┌─────────────────────────────────────────────────────────────────────┐
│                        MICHI LINK ECOSYSTEM                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────────┐        ┌──────────────────────────────┐   │
│  │    michi-link-server │◄──────►│      michi-link-client       │   │
│  │  (Núcleo del sistema)│        │  (App móvil / escritorio)    │   │
│  └──────────┬───────────┘        └──────────────┬───────────────┘   │
│             │                                    │                   │
│             │       ┌──────────────────┐         │                   │
│             └──────►│  michi-link-web  │◄────────┘                   │
│                     │  (Interfaz web)  │                             │
│                     └──────────────────┘                             │
│                              │                                       │
│                              ▼                                       │
│                     ┌──────────────────┐                             │
│                     │ michi-link-cli   │                             │
│                     │  (Línea de       │                             │
│                     │   comandos)      │                             │
│                     └──────────────────┘                             │
│                              │                                       │
│                              ▼                                       │
│                     ┌──────────────────┐                             │
│                     │  michi-link-     │                             │
│                     │  receiver-nim    │                             │
│                     │  (Firmware       │                             │
│                     │   para speakers) │                             │
│                     └──────────────────┘                             │
│                                                                     │
│              ◄─────────── Descubrimiento mDNS ──────────────────►   │
│              ◄─────────── Streaming audio ─────────────────────►   │
│              ◄─────────── Sincronización multiroom ────────────►    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Los 5 proyectos del ecosistema

### 1. `michi-link-server`

**Rol:** Núcleo del sistema. Servidor central que gestiona la biblioteca, la autenticación, la sincronización y orquesta la reproducción.

**Responsabilidades:**
- Servir la API REST y WebSocket.
- Gestionar la biblioteca de música (escaneo, metadatos, almacenamiento).
- Autenticar y autorizar dispositivos via pairing.
- Coordinar la reproducción multiroom.
- Mantener el manifiesto de sincronización.
- Descubrir receptores en la red via mDNS.
- Servir streaming de audio y carátulas.

**Tecnologías:** Nim, HTTP/1.1, WebSockets, SQLite, mDNS.

---

### 2. `michi-link-client`

**Rol:** Aplicación cliente móvil y de escritorio. Interfaz principal para el usuario final.

**Responsabilidades:**
- Navegar y buscar la biblioteca musical.
- Controlar la reproducción (play, pause, skip, volumen).
- Gestionar playlists y favoritos.
- Descargar tracks para reproducción offline.
- Emparejarse con el servidor via pairing.
- Mostrar estado de reproducción en tiempo real.
- Controlar receptores y salas multiroom.

**Tecnologías:** Kotlin Multiplatform / Flutter / React Native (según la implementación).

---

### 3. `michi-link-web`

**Rol:** Interfaz web administrativa y de control. Accesible desde cualquier navegador en la red local.

**Responsabilidades:**
- Dashboard de estado del servidor.
- Gestión de biblioteca (escaneo, edición de metadatos).
- Administración de dispositivos emparejados.
- Control básico de reproducción.
- Visualización de estadísticas.

**Tecnologías:** React / Vue.js / Svelte (según la implementación).

---

### 4. `michi-link-cli`

**Rol:** Herramienta de línea de comandos para administración y automatización.

**Responsabilidades:**
- Scriptear operaciones de biblioteca.
- Automatizar escaneos y sincronización.
- Administrar dispositivos y pares.
- Integración con pipelines CI/CD y sistemas de automatización del hogar.

**Tecnologías:** Nim / Python / Shell.

---

### 5. `michi-link-receiver-nim`

**Rol:** Firmware para dispositivos de reproducción (speakers inteligentes, amplificadores, Raspberry Pi con DAC).

**Responsabilidades:**
- Recibir y reproducir streaming de audio.
- Reportar estado y heartbeat al servidor.
- Sincronización multiroom (audio sincronizado entre múltiples receptores).
- Control de volumen local.
- Actualización de firmware OTA.

**Tecnologías:** Nim, ALSA / PulseAudio, mDNS.

---

## Cómo Michi Link conecta los proyectos

```
                        michi-link-web
                             │
                    HTTP API (navegador)
                             │
michi-link-client ──HTTP API/WS──► michi-link-server ◄──HTTP API── michi-link-cli
                                        │
                                  v1-lite API (HTTP)
                                        │
                                        ▼
                             michi-link-receiver-nim
                                  (reproductor)
```

El servidor actúa como el orquestador central:
- **Clientes** (app móvil, web, CLI) se comunican con el servidor via la **API REST v1** y **WebSockets** para eventos en tiempo real.
- **Receptores** se comunican con el servidor via la **API v1-lite** (endpoints más simples, pensados para firmware con recursos limitados).
- El **descubrimiento** inicial se realiza mediante **mDNS** (Bonjour/Avahi) en la red local.
- El **streaming** de audio fluye desde el servidor hacia los receptores, ya sea directo o mediante relay.

---

## Modelo de identidad de dispositivo

Cada dispositivo en el ecosistema tiene una identidad única representada por:

```json
{
  "device_id": "uuid-unico",
  "device_name": "Cocina Speaker",
  "device_type": "speaker",
  "roles": ["receiver", "player"],
  "capabilities": {
    "streaming_formats": ["flac", "mp3", "ogg"],
    "max_bitrate": 320,
    "max_sample_rate": 192000,
    "multiroom": true,
    "transcoding": false,
    "sync": true
  }
}
```

### Campos

| Campo           | Tipo     | Descripción                                              |
|-----------------|----------|----------------------------------------------------------|
| `device_id`     | UUID     | Identificador único del dispositivo, generado en pairing.|
| `device_name`   | string   | Nombre legible asignado por el usuario.                  |
| `device_type`   | enum     | `server`, `mobile`, `desktop`, `speaker`, `amplifier`, `cli`, `web`. |
| `roles`         | string[] | Lista de roles que el dispositivo desempeña.             |
| `capabilities`  | object   | Mapa de capacidades técnicas del dispositivo.            |

---

## Roles

| #  | Rol                  | Descripción                                                    |
|----|----------------------|----------------------------------------------------------------|
| 1  | `core`               | Núcleo del sistema. Ejecuta el servidor central.               |
| 2  | `controller`         | Controla la reproducción (play, pause, skip, volumen).         |
| 3  | `player`             | Capaz de iniciar y gestionar sesiones de reproducción.         |
| 4  | `receiver`           | Recibe y reproduce audio (speaker físico o virtual).           |
| 5  | `sync_leader`        | Lidera la sincronización multiroom.                            |
| 6  | `sync_follower`      | Sigue la sincronización de otro dispositivo.                   |
| 7  | `library_service`    | Provee acceso a la biblioteca de música.                      |
| 8  | `library_consumer`   | Consume la biblioteca (navegación, búsqueda, descarga).        |
| 9  | `library_admin`      | Administra la biblioteca (escaneo, edición de metadatos).      |
| 10 | `auth_service`       | Provee servicios de autenticación y autorización.              |
| 11 | `auth_consumer`      | Consume servicios de autenticación.                            |
| 12 | `pairing_initiator`  | Inicia el flujo de emparejamiento.                             |
| 13 | `pairing_responder`  | Responde y confirma el emparejamiento.                         |
| 14 | `stream_source`      | Fuente de streaming de audio.                                  |
| 15 | `stream_sink`        | Receptor de streaming de audio.                                |
| 16 | `websocket_publisher`| Publica eventos via WebSocket.                                 |
| 17 | `websocket_subscriber`| Suscribe a eventos via WebSocket.                             |
| 18 | `discovery_agent`    | Participa en el descubrimiento mDNS.                           |

---

## Permisos

| #  | Permiso                       | Descripción                                                |
|----|-------------------------------|------------------------------------------------------------|
| 1  | `server.info.read`            | Leer información del servidor.                             |
| 2  | `server.status.read`          | Leer estado de salud del servidor.                         |
| 3  | `pairing.start`               | Iniciar proceso de emparejamiento.                         |
| 4  | `pairing.confirm`             | Confirmar emparejamiento.                                  |
| 5  | `token.refresh`               | Renovar tokens de autenticación.                           |
| 6  | `devices.revoke`              | Revocar acceso de dispositivos.                            |
| 7  | `library.stats.read`          | Leer estadísticas de la biblioteca.                        |
| 8  | `library.scan`                | Iniciar escaneo de biblioteca.                             |
| 9  | `tracks.read`                 | Leer información de tracks.                                |
| 10 | `albums.read`                 | Leer información de álbumes.                               |
| 11 | `artists.read`                | Leer información de artistas.                              |
| 12 | `search.execute`              | Ejecutar búsquedas globales.                               |
| 13 | `stream.audio`                | Hacer streaming de audio.                                  |
| 14 | `download.track`              | Descargar tracks para offline.                             |
| 15 | `artwork.read`                | Leer carátulas.                                            |
| 16 | `playlists.read`              | Leer playlists.                                            |
| 17 | `playlists.create`            | Crear playlists.                                           |
| 18 | `playlists.update`            | Actualizar playlists.                                      |
| 19 | `playlists.delete`            | Eliminar playlists.                                        |
| 20 | `playlists.tracks.write`      | Modificar tracks en playlists.                             |
| 21 | `sync.read`                   | Leer manifiestos de sincronización.                        |
| 22 | `sync.write`                  | Subir estado de sincronización.                            |
| 23 | `playback.state.read`         | Leer estado de reproducción.                               |
| 24 | `playback.control`            | Controlar la reproducción.                                 |
| 25 | `queue.read`                  | Leer la cola de reproducción.                              |
| 26 | `queue.write`                 | Modificar la cola de reproducción.                         |
| 27 | `receivers.control`           | Controlar receptores.                                      |
| 28 | `rooms.manage`                | Gestionar salas multiroom.                                 |

---

## Flujos de comunicación

### 1. Descubrimiento → Pairing → Comunicación autorizada

```
CLIENTE                              SERVIDOR
   │                                     │
   │  1. mDNS: ¿Hay servidores?          │
   │────────────────────────────────────►│
   │                                     │
   │  2. mDNS: Aquí estoy (server.info)  │
   │◄────────────────────────────────────│
   │                                     │
   │  3. POST /pair/start                │
   │  {device_name, device_type, roles,  │
   │   auth_strategy, michi_id,          │
   │   public_key, challenge_nonce,      │
   │   challenge_signature}              │
   │────────────────────────────────────►│
   │                                     │
   │  4. {session_id, expires_at,        │
   │     attempts_remaining,             │
   │     server_michi_id,                │
   │     server_public_key}              │
   │◄────────────────────────────────────│
   │                                     │
   │  5. (Usuario lee el PIN de 6        │
   │     dígitos en la pantalla del      │
   │     servidor — nunca viaja por la   │
   │     red)                            │
   │                                     │
   │  6. POST /pair/confirm              │
   │  {session_id, pin, michi_id,        │
   │   public_key}                       │
   │────────────────────────────────────►│
   │                                     │
   │  7. {token, refresh_token?,         │
   │     expires_in, device_id,          │
   │     server_id}                      │
   │◄────────────────────────────────────│
   │                                     │
   │  8. GET /tracks (con Bearer token)  │
   │────────────────────────────────────►│
   │                                     │
   │  9. {data: [...], page, total, ...} │
   │◄────────────────────────────────────│
```

**Pasos:**
1. El cliente descubre el servidor en la red local via **mDNS**.
2. El servidor responde con su información (`server.info`).
3. El cliente inicia el pairing enviando su identidad y un **challenge Ed25519** (firma sobre los bytes crudos del nonce, que prueba posesión de la clave).
4. El servidor verifica la firma, crea la sesión (5 min, 5 intentos, uso único) y devuelve la sesión con la identidad del servidor. El **PIN de 6 dígitos se muestra en el servidor** y nunca viaja por la red.
5. El usuario lee el PIN en el dispositivo servidor.
6. El cliente confirma el pairing con `session_id` + `pin`.
7. El servidor entrega un token Bearer opaco (y refresh token si lo soporta).
8. En adelante, todas las solicitudes se realizan con el token en el header `Authorization`.

---

### 2. Autenticación

```
┌─────────┐         ┌──────────────┐         ┌──────────┐
│ Cliente │         │ Auth Service │         │ Token DB│
└────┬────┘         └──────┬───────┘         └────┬─────┘
     │                     │                      │
     │ POST /pair/start    │                      │
     │ {michi_id,          │                      │
     │  public_key,        │                      │
     │  challenge_nonce,   │                      │
     │  challenge_sig}     │                      │
     │────────────────────►│                      │
     │                     │ Valida challenge     │
     │                     │ (firma Ed25519 sobre │
     │                     │ bytes crudos del     │
     │                     │ nonce)               │
     │                     │ Crea sesión + PIN    │
     │                     │ (5 min, 5 intentos,  │
     │                     │ uso único)           │
     │                     │──────►               │
     │ session_id, expires │                      │
     │◄────────────────────│                      │
     │                     │ (PIN mostrado en el  │
     │                     │ servidor, nunca en   │
     │                     │ el wire)             │
     │                     │                      │
     │ POST /pair/confirm  │                      │
     │ {session_id, pin}   │                      │
     │────────────────────►│                      │
     │                     │ Valida PIN           │
     │                     │ (constante, keyed)   │
     │                     │──────►               │
     │                     │◄──────               │
     │                     │                      │
     │                     │ Genera token opaco   │
     │                     │ (device_id, roles,   │
     │                     │  permissions, exp)   │
     │                     │──────►               │
     │ {token, refresh}    │                      │
     │◄────────────────────│                      │
     │                     │                      │
     │ GET /tracks         │                      │
     │ Authorization: Bearer <token>              │
     │───────────────────────────────────────────►│
     │                     │                      │
     │                     │ Valida token        │
     │                     │ Verifica permisos    │
     │                     │◄─────────────────────│
     │ {data: [...]}       │                      │
     │◄───────────────────────────────────────────│
```

**Flujo:**
- El token es un bearer token **opaco** (no JWT): un valor aleatorio resuelto server-side con `device_id`, `roles` y `permissions` asociados.
- El servidor valida el token en cada solicitud.
- Los permisos se verifican contra el endpoint solicitado.
- Los tokens expiran (por defecto en 1 hora) y se renuevan via `/token/refresh`.

---

### 3. Sincronización

```
CLIENTE                              SERVIDOR
   │                                     │
   │  GET /sync/manifest                 │
   │────────────────────────────────────►│
   │                                     │
   │  {manifest_id, tracks: {id: vers..}}│
   │◄────────────────────────────────────│
   │                                     │
   │  (Cliente compara con su estado     │
   │   local y determina diferencias)    │
   │                                     │
   │  GET /sync/manifest/delta           │
   │  ?since=2026-06-28T12:00:00Z        │
   │────────────────────────────────────►│
   │                                     │
   │  {changes: {tracks: {added,         │
   │   updated, removed}, albums: ...}}  │
   │◄────────────────────────────────────│
   │                                     │
   │  (Cliente aplica cambios locales)   │
   │                                     │
   │  POST /sync/state                   │
   │  {device_id, manifest_id,           │
   │   downloaded_tracks,                │
   │   storage_used_bytes}               │
   │────────────────────────────────────►│
   │                                     │
   │  {success: true, acknowledged_at}   │
   │◄────────────────────────────────────│
```

**Objetivo:** Mantener una copia offline coherente de la biblioteca en los dispositivos clientes.

- El **manifiesto completo** contiene todos los elementos con sus versiones.
- El **manifiesto delta** solo incluye cambios desde la última sincronización.
- El cliente reporta su estado para que el servidor pueda rastrear qué dispositivos están al día.

---

### 4. Streaming

```
CLIENTE / RECEPTOR                     SERVIDOR
   │                                        │
   │  GET /stream/{track_id}                │
   │  Range: bytes=0-                       │
   │  Accept: audio/flac                    │
   │──────────────────────────────────────► │
   │                                        │
   │  206 Partial Content                   │
   │  Content-Type: audio/flac              │
   │  Content-Range: bytes 0-1023/35000000  │
   │  Accept-Ranges: bytes                  │
   │◄───────────────────────────────────────│
   │                                        │
   │  (Cliente bufferiza y reproduce)       │
   │                                        │
   │  GET /stream/{track_id}                │
   │  Range: bytes=1024-                    │
   │──────────────────────────────────────► │
   │                                        │
   │  206 Partial Content                   │
   │  Content-Range: bytes 1024-2047/35000000
   │◄───────────────────────────────────────│
   │                                        │
   │  (Continúa hasta completar el archivo  │
   │   o hasta que el usuario detiene)      │
```

**Características:**
- Soporte para **peticiones parciales (Range)** para búsqueda y reanudación.
- El cliente puede solicitar el formato que prefiera via `Accept` header.
- El servidor puede transcodificar sobre la marcha si es necesario.
- El streaming es **stateless**: cada petición es independiente.
- Para receptores, el servidor puede enviar la URL de stream directamente.

---

### 5. Control

```
┌──────────┐     ┌─────────┐     ┌───────────┐
│ Cliente  │     │ Servidor│     │ Receptor  │
└────┬─────┘     └────┬────┘     └─────┬─────┘
     │                │                │
     │ POST /playback/control          │
     │ {action: "play"}               │
     │───────────────►                │
     │                │                │
     │                │ POST /receiver │
     │                │ /session/start │
     │                │ {stream_url,   │
     │                │  token, vol...}│
     │                │───────────────►│
     │                │                │
     │                │ {success,      │
     │                │  session_id,   │
     │                │  state}        │
     │                │◄───────────────│
     │                │                │
     │ {success,      │                │
     │  state,        │                │
     │  position}     │                │
     │◄───────────────│                │
     │                │                │
     │ (Cada 30s heartbeat)            │
     │                │◄───────────────│
     │                │ {device_id,    │
     │                │  state, vol,   │
     │                │  position}     │
     │                │                │
     │ WebSocket:     │                │
     │ playback.      │                │
     │ state_changed  │                │
     │◄───────────────│                │
```

**Flujo de control:**
1. El cliente envía un comando de control al servidor.
2. El servidor traduce el comando a la API v1-lite del receptor.
3. El receptor ejecuta la acción y responde.
4. El servidor confirma al cliente.
5. El receptor envía heartbeats periódicos con su estado.
6. El servidor propaga cambios de estado via WebSocket a todos los suscriptores.

---

### 6. Multiroom

```
SERVIDOR
   │
   ├──► RECEPTOR A (Cocina)
   │    │   Stream URL: /stream/uuid-track?sync=true
   │    │   Sync Leader
   │    │
   ├──► RECEPTOR B (Salón)
   │    │   Stream URL: /stream/uuid-track?sync=true
   │    │   Sync Follower (delay = offset calculado)
   │    │
   └──► RECEPTOR C (Comedor)
        │   Stream URL: /stream/uuid-track?sync=true
            Sync Follower (delay = offset calculado)
```

**Flujo:**
1. El usuario crea una **sala** (room) que agrupa múltiples receptores.
2. El usuario inicia reproducción en la sala via `POST /rooms/{id}/play`.
3. El servidor inicia sesiones en cada receptor miembro.
4. Un receptor actúa como **sync_leader** (referencia de tiempo).
5. Los demás receptores actúan como **sync_followers**.
6. El servidor calcula delays para compensar latencias de red.
7. El audio se reproduce sincronizado en todos los miembros.

**Sincronización:**
- El sync_leader envía su timeline de reproducción.
- Los sync_followers calculan el offset necesario.
- Se realizan ajustes periódicos para mantener la sincronía.
- El margen típico es < 10ms entre dispositivos.

---

## Estrategia de compatibilidad de versiones

### Versionado semántico

Todos los proyectos del ecosistema siguen **SemVer 2.0**: `MAJOR.MINOR.PATCH`.

### API v1

- La API v1 es estable. Los cambios rompientes requieren una nueva versión mayor (`v2`).
- Los campos en las respuestas JSON pueden ser añadidos (nuevas claves) en versiones menores.
- Los campos existentes no serán eliminados ni renombrados dentro de v1.
- Los endpoints nuevos pueden agregarse en versiones menores.
- Los parámetros opcionales nuevos pueden agregarse en versiones menores.

### Matriz de compatibilidad

| Servidor ↓ \ Cliente → | v1.0.x | v1.1.x | v1.2.x | v2.0.x |
|------------------------|--------|--------|--------|--------|
| v1.0.x                 | ✅     | ✅     | ✅     | ❌     |
| v1.1.x                 | ✅     | ✅     | ✅     | ❌     |
| v1.2.x                 | ✅     | ✅     | ✅     | ❌     |
| v2.0.x                 | ❌     | ❌     | ❌     | ✅     |

### Reglas

- **Dentro de la misma versión mayor:** Compatibilidad total hacia adelante y atrás para el mismo rango MINOR. Un servidor v1.2.x puede servir a un cliente v1.0.x siempre que el cliente ignore los campos nuevos.
- **Entre versiones mayores:** No hay compatibilidad garantizada. La API puede cambiar completamente.
- **Receptores (firmware):** Los receptores deben ser compatibles con al menos una versión mayor completa del servidor. Los receptores v1 pueden funcionar con servidores v2 si el servidor implementa un adaptador de compatibilidad.

### Negociación de versión

- El servidor expone `api_version` en `GET /server/info`.
- El cliente debe verificar la versión antes de operar.
- Si hay incompatibilidad, el cliente debe informar al usuario y sugerir actualizar.

### Deprecación

- Los endpoints deprecados se anuncian con el header `Deprecation: true`.
- Se mantienen por al menos **2 versiones menores** antes de ser eliminados.
- La documentación marca claramente los endpoints deprecados.
