# Capability Probing — Michi Link API (Future/Optional)

## Propósito

Definir un mecanismo opcional para que los clientes descubran dinámicamente las capacidades de un servidor sin depender de campos fijos en `server/info`.

## Estado

**Este documento es un blueprint futuro.** No es necesario para v1.0.0-alpha ni beta. Se implementará en v1.1 o v2 si hay demanda.

## Endpoint Propuesto: GET /api/v1/capabilities

Consulta estática de capacidades. Complementa `features` en `server/info`.

**Respuesta:**

```json
{
  "api_version": "v1",
  "service": "michi-micro-server",
  "roles": ["music_server", "library_host", "playback_host"],
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
  "capabilities": {
    "streaming": {
      "formats": ["flac", "mp3", "ogg", "aac", "wav"],
      "max_bitrate": 1411,
      "max_sample_rate": 192000,
      "transcoding": true,
      "range_support": true
    },
    "sync": {
      "manifest": true,
      "delta": true,
      "cursor": true,
      "legacy_since": true,
      "legacy_manifest_id": true
    },
    "playback": {
      "commands": ["play", "pause", "toggle", "stop", "next", "previous", "seek", "set_volume", "mute", "unmute", "shuffle", "repeat"],
      "legacy_action": true
    }
  }
}
```

## Endpoint Propuesto: POST /api/v1/capabilities/probe

Prueba dinámica de una capacidad específica. Útil para verificar que una funcionalidad realmente funciona, no solo está declarada.

**Request:**

```json
{
  "probe": "stream_range",
  "params": { "track_id": "uuid" }
}
```

**Response:**

```json
{
  "probe": "stream_range",
  "status": "pass",
  "detail": "HTTP 206 Partial Content con Range bytes=0-1023",
  "duration_ms": 45
}
```

## Integración con features booleanas

- `features` en `server/info` sigue siendo booleano simple para v1.0.x.
- `capabilities` es el lugar para detalles extendidos.
- Un servidor puede tener `features.streaming: true` y exponer detalles en `/capabilities`.

## Uso esperado

- Mobile consulta `/capabilities` después de pairar para saber qué formatos de stream están disponibles.
- Player consulta `/capabilities` de Micro Server para saber si soporta transcoding.
- No es blocker para beta.
