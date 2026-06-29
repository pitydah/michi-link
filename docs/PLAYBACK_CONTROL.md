# Protocolo de Control de Reproducción

## Estado de Reproducción

El estado de reproducción se representa con el siguiente objeto:

| Campo | Tipo | Descripción |
|-------|------|-------------|
| state | string | `playing`, `paused`, `stopped`, `loading` |
| track_id | string | ID de la pista actual |
| position_ms | integer | Posición actual en milisegundos |
| duration_ms | integer | Duración total en milisegundos |
| volume | integer | Volumen 0–100 |
| shuffle | boolean | Si el modo aleatorio está activado |
| repeat | string | `off`, `one` o `all` |
| device_id | string | ID del dispositivo que controla la reproducción |

### Ejemplo

```json
{
  "state": "playing",
  "track_id": "a1b2c3d4",
  "position_ms": 45000,
  "duration_ms": 240000,
  "volume": 72,
  "shuffle": false,
  "repeat": "off",
  "device_id": "device-abc-123"
}
```

## Comandos de Control

### Payload oficial

El campo oficial es **`command`**. El campo `action` se acepta como legacy/fallback temporal.

| Comando | Valor de `position_ms` | Valor de `volume` | Descripción |
|---------|----------------------|-------------------|-------------|
| play | opcional (int) | — | Iniciar o reanudar reproducción |
| pause | — | — | Pausar reproducción |
| toggle | — | — | Alternar play/pausa |
| stop | — | — | Detener reproducción |
| next | — | — | Siguiente pista |
| previous | — | — | Pista anterior |
| seek | requerido (int ≥ 0) | — | Ir a posición específica |
| set_volume | — | requerido (int 0–100) | Establecer volumen |
| mute | — | — | Silenciar |
| unmute | — | — | Reactivar sonido |
| shuffle | — | — | Activar/desactivar aleatorio |
| repeat | — | — | Cambiar modo de repetición |

> El campo `value` queda permitido solo como fallback legacy. Las nuevas implementaciones DEBEN usar `position_ms` para seek y `volume` para set_volume.

### Ejemplo: seek

```json
{
  "command": "seek",
  "position_ms": 90000
}
```

### Ejemplo: volumen

```json
{
  "command": "set_volume",
  "volume": 70
}
```

### Ejemplo: play

```json
{
  "command": "play",
  "position_ms": 0
}
```

### Ejemplo respuesta

```json
{
  "success": true,
  "state": "playing",
  "position_ms": 90000
}
```

## Sesión de Reproducción

Una sesión de reproducción define qué dispositivo o cliente controla la salida de audio. Cada sesión tiene un único propietario.

### Propietario de la Sesión

- El propietario es quien inició la reproducción.
- Solo el propietario puede enviar comandos de control.
- Si el propietario se desconecta, la sesión puede:
  - Transferirse a otro dispositivo.
  - Mantenerse hasta timeout.
  - Detenerse.

### Transferencia de Propietario

```json
{
  "command": "transfer",
  "device_id": "device-xyz-789"
}
```

El dispositivo destino debe aceptar explícitamente la transferencia.

## Modelo de Control Remoto

Un dispositivo móvil puede controlar la reproducción en un servidor o desktop sin necesidad de gestionar el audio localmente.

```
[ Móvil ] --POST /playback/control--> [ Servidor/Desktop ]
                                          |
                                      [ Altavoces ]
```

El móvil envía comandos vía HTTP POST a `/playback/control`. El servidor ejecuta la reproducción y notifica cambios de estado a través de WebSocket `/events`.

## Gestión de Cola

### Estructura de un Elemento de la Cola

| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | string | ID único del elemento en la cola |
| track_id | string | ID de la pista |
| track | object | Datos completos de la pista |
| added_by | string | ID del usuario que la agregó |
| position | integer | Posición en la cola |
| added_at | string | Fecha ISO 8601 de agregado |

### Comandos de Cola

Los comandos de cola se envían a los endpoints REST dedicados (`/queue/items`, `/queue/jump`, `/queue/reorder`, `/queue/items/{id}`), no a `/playback/control`.

## Modos de Repetición

| Modo | Comportamiento |
|------|----------------|
| off | No repetir. Al terminar la cola, se detiene. |
| one | Repetir la pista actual infinitamente. |
| all | Repetir toda la cola al llegar al final. |
