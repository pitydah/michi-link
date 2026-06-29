# Receivers Físicos — Protocolo v1-lite

## ¿Qué es v1-lite?

v1-lite es un subconjunto simplificado del protocolo v1 diseñado para dispositivos físicos con recursos limitados (receptores de audio). Estos dispositivos no gestionan bibliotecas, listas de reproducción ni sincronización de estado compleja.

### Limitaciones de los Receivers

- No administran biblioteca musical.
- No manejan playlists.
- No participan en sincronización de estado entre pares.
- Solo ejecutan órdenes del servidor.

### Capacidades

- Anunciar presencia en la red.
- Aceptar pareado con un servidor.
- Recibir sesión de audio (URL + formato).
- Controlar volumen.
- Enviar heartbeat.
- Actualizar firmware.

## Endpoints v1-lite

### Anuncio y Descubrimiento

#### `POST /api/v1/receivers/announce`

Anuncio inicial o heartbeat enviado por el receiver.

**Solicitud:**
```json
{
  "id": "rec-std-001",
  "name": "Receiver Salón",
  "model": "standard",
  "version": "1.0.0",
  "capabilities": ["audio", "volume", "heartbeat"],
  "ip": "192.168.1.50",
  "port": 9001
}
```

**Respuesta:**
```json
{
  "status": "paired",
  "server_id": "srv-michi-001",
  "session": null
}
```

#### `GET /api/v1/receivers/{id}`

Obtener información del receiver.

**Respuesta Standard:**
```json
{
  "id": "rec-std-001",
  "name": "Receiver Standard",
  "model": "standard",
  "version": "1.0.0",
  "capabilities": ["audio", "volume", "heartbeat"],
  "status": "idle",
  "volume": 50,
  "firmware": {
    "current": "1.0.0",
    "latest": "1.0.2"
  },
  "hardware": {
    "audio_output": "jack_3.5mm",
    "dac": "integrado",
    "ram_kb": 512,
    "storage_kb": 4096
  }
}
```

**Respuesta Hi-Fi:**
```json
{
  "id": "rec-hifi-001",
  "name": "Receiver Hi-Fi",
  "model": "hifi",
  "version": "2.1.0",
  "capabilities": ["audio", "volume", "heartbeat", "high_res"],
  "status": "playing",
  "volume": 65,
  "firmware": {
    "current": "2.1.0",
    "latest": "2.2.0"
  },
  "hardware": {
    "audio_output": "rca",
    "dac": "pcm5242",
    "sample_rates": [44100, 48000, 96000],
    "bit_depth": 24,
    "ram_kb": 4096,
    "storage_kb": 16384
  }
}
```

### Heartbeat

El receiver envía heartbeat cada **10 segundos**. Si el servidor no recibe heartbeat durante **30 segundos**, el receiver se considera desconectado.

```
Cada 10s → POST /api/v1/receivers/{id}/heartbeat
           { "timestamp": "2026-06-29T12:00:00Z" }
           
Si 30s sin heartbeat → estado: disconnected
```

### Sesión de Audio

#### `POST /api/v1/receivers/{id}/session/start`

El servidor indica al receiver que inicie reproducción.

**Solicitud:**
```json
{
  "stream_url": "http://192.168.1.10:8000/stream/abc123",
  "format": "flac",
  "sample_rate": 48000,
  "bit_depth": 16,
  "channels": 2,
  "volume": 70
}
```

**Respuesta:**
```json
{
  "status": "playing",
  "session_id": "sess-001"
}
```

#### `POST /api/v1/receivers/{id}/session/stop`

El servidor ordena detener la reproducción.

**Solicitud:**
```json
{}
```

**Respuesta:**
```json
{
  "status": "stopped"
}
```

### Control de Volumen

#### `PUT /api/v1/receivers/{id}/volume`

**Solicitud:**
```json
{
  "volume": 75
}
```

**Respuesta:**
```json
{
  "status": "ok",
  "volume": 75
}
```

Rango: 0–100 (entero).

### Actualización de Firmware

#### `GET /api/v1/receivers/{id}/firmware`

Obtener información de firmware disponible.

**Respuesta:**
```json
{
  "current": "1.0.0",
  "latest": "1.0.2",
  "changelog_url": "http://firmware.michi-link.local/receivers/v1.0.2/CHANGELOG.md",
  "update_available": true
}
```

#### `POST /api/v1/receivers/{id}/firmware/update`

Iniciar actualización de firmware.

**Solicitud:**
```json
{
  "url": "http://firmware.michi-link.local/receivers/v1.0.2/firmware.bin",
  "checksum": "sha256:a1b2c3d4..."
}
```

**Respuesta:**
```json
{
  "status": "updating",
  "estimated_time_seconds": 120
}
```

## Anuncio UDP

Los receivers también se anuncian vía UDP en el mismo puerto de descubrimiento (`6666`).

```
RECEIVER_ANNOUNCE
{
  "type": "receiver.announce",
  "id": "rec-std-001",
  "name": "Receiver Salón",
  "model": "standard",
  "ip": "192.168.1.50",
  "port": 9001,
  "version": "1.0.0"
}
```

## Notas de Hardware

### Standard

| Característica  | Especificación                    |
|-----------------|-----------------------------------|
| Salida de audio | Jack 3.5mm estéreo                |
| DAC             | Integrado en SoC                  |
| RAM             | ~512 KB                           |
| Almacenamiento  | ~4 MB                             |
| Formato máximo  | 16-bit / 48 kHz                   |
| Conectividad    | Wi-Fi 802.11 b/g/n                |
| Uso típico      | Habitaciones secundarias, baño    |

### Hi-Fi

| Característica  | Especificación                       |
|-----------------|--------------------------------------|
| Salida de audio | RCA (L/R) + Jack 3.5mm              |
| DAC             | PCM5242 o superior                   |
| RAM             | ~4 MB                                |
| Almacenamiento  | ~16 MB                               |
| Formatos        | Hasta 24-bit / 96 kHz               |
| Conectividad    | Wi-Fi 802.11 b/g/n + Ethernet       |
| Uso típico      | Sala de estar, equipo principal      |
