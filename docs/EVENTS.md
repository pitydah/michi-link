# Eventos WebSocket

## Endpoint

```
GET /api/v1/events
```

Conexión WebSocket persistente. El servidor envía eventos en tiempo real a todos los clientes conectados.

## Formato del Evento

```json
{
  "type": "event_type",
  "data": {},
  "timestamp": "2026-06-29T12:00:00Z"
}
```

| Campo     | Tipo   | Descripción                      |
|-----------|--------|----------------------------------|
| type      | string | Tipo de evento                   |
| data      | object | Datos específicos del evento     |
| timestamp | string | Marca de tiempo ISO 8601         |

## Tipos de Evento

### Reproducción

| Evento                   | Descripción                     | data                                      |
|--------------------------|---------------------------------|-------------------------------------------|
| `playback.state_changed` | Cambió el estado de reproducción | `{state, track_id, position_ms, ...}`     |
| `queue.updated`          | La cola fue modificada          | `{queue: [...]}`                          |

### Biblioteca

| Evento                       | Descripción                       | data                              |
|------------------------------|-----------------------------------|-----------------------------------|
| `library.scan_started`       | Escaneo de biblioteca iniciado    | `{path, total_files}`            |
| `library.scan_completed`     | Escaneo de biblioteca finalizado  | `{tracks_found, duration_ms}`    |
| `library.tracks_added`       | Pistas agregadas a la biblioteca  | `{track_ids: [...]}`             |
| `library.tracks_removed`     | Pistas eliminadas de la biblioteca| `{track_ids: [...]}`             |

### Dispositivos

| Evento                 | Descripción                          | data                              |
|------------------------|--------------------------------------|-----------------------------------|
| `device.paired`        | Dispositivo emparejado               | `{device_id, name, type}`        |
| `device.unpaired`      | Dispositivo desemparejado            | `{device_id}`                    |
| `device.discovered`    | Nuevo dispositivo descubierto en red | `{device_id, name, ip, type}`    |
| `device.lost`          | Dispositivo perdido (timeout)        | `{device_id}`                    |

### Receivers

| Evento                         | Descripción                        | data                              |
|--------------------------------|------------------------------------|-----------------------------------|
| `receiver.session_started`     | Sesión de audio iniciada           | `{receiver_id, session_id, url}` |
| `receiver.session_stopped`     | Sesión de audio detenida           | `{receiver_id, session_id}`      |
| `receiver.volume_changed`      | Volumen del receiver modificado    | `{receiver_id, volume}`          |
| `receiver.disconnected`        | Receiver desconectado (heartbeat)  | `{receiver_id, last_seen}`       |

### Rooms

| Evento                   | Descripción                           | data                              |
|--------------------------|---------------------------------------|-----------------------------------|
| `room.updated`           | Room modificada (nombre, dispositivos)| `{room_id, name, ...}`           |
| `room.playback_changed`  | Reproducción en room cambiada         | `{room_id, is_playing, track}`    |

### Cadenas de Audio

| Evento           | Descripción                        | data                              |
|------------------|------------------------------------|-----------------------------------|
| `chain.updated`  | Cadena de audio modificada         | `{chain_id, name, ...}`          |

### Servidor

| Evento                   | Descripción                     | data                              |
|--------------------------|---------------------------------|-----------------------------------|
| `server.shutting_down`   | El servidor se está apagando    | `{reason, grace_period_ms}`      |

## Ejemplo Completo

```json
{
  "type": "playback.state_changed",
  "data": {
    "state": "playing",
    "track_id": "a1b2c3d4",
    "position_ms": 0,
    "duration_ms": 240000,
    "volume": 70,
    "shuffle": false,
    "repeat": "off"
  },
  "timestamp": "2026-06-29T12:00:00Z"
}
```

## Estrategia de Reconexión

1. El cliente debe intentar reconectar inmediatamente tras una desconexión.
2. Si falla, esperar 1 segundo y reintentar.
3. Duplicar el tiempo de espera en cada reintento (backoff exponencial): 1s, 2s, 4s, 8s... hasta un máximo de 30s.
4. Al reconectar, el servidor envía el estado actual completo de reproducción como un evento `playback.state_changed` inicial.

```
Intento 1: 0s
Intento 2: 1s
Intento 3: 2s
Intento 4: 4s
Intento 5: 8s
Intento 6: 16s
Intento 7+: 30s (máximo)
```

## Filtrado de Eventos por Tipo

El cliente puede suscribirse solo a ciertos tipos de eventos enviando un mensaje al conectar:

```json
{
  "action": "subscribe",
  "events": ["playback.state_changed", "queue.updated", "room.updated"]
}
```

Para cancelar suscripción:

```json
{
  "action": "unsubscribe",
  "events": ["room.updated"]
}
```

Si no se envía suscripción, el cliente recibe todos los eventos.
