# Protocolo de Control de Reproducción

## Estado de Reproducción

El estado de reproducción se representa con el siguiente objeto:

| Campo         | Tipo    | Descripción                                      |
|---------------|---------|--------------------------------------------------|
| state         | string  | `playing`, `paused` o `stopped`                  |
| track_id      | string  | ID de la pista actual                            |
| position_ms   | integer | Posición actual en milisegundos                  |
| duration_ms   | integer | Duración total en milisegundos                   |
| volume        | integer | Volumen 0–100                                    |
| shuffle       | boolean | Si el modo aleatorio está activado               |
| repeat        | string  | `off`, `one` o `all`                             |
| device_id     | string  | ID del dispositivo que controla la reproducción  |

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

| Comando         | Parámetros                         | Descripción                           |
|-----------------|------------------------------------|---------------------------------------|
| play            | `track_id` (opcional)              | Iniciar o reanudar reproducción       |
| pause           | —                                  | Pausar reproducción                   |
| stop            | —                                  | Detener reproducción                  |
| next            | —                                  | Siguiente pista                       |
| previous        | —                                  | Pista anterior                        |
| seek            | `position_ms`                      | Ir a posición específica              |
| set_volume      | `volume` (0–100)                   | Establecer volumen                    |
| toggle_shuffle  | —                                  | Activar/desactivar aleatorio          |
| toggle_repeat   | —                                  | Cambiar modo de repetición            |

### Ejemplo de Solicitud

```json
{
  "command": "seek",
  "params": {
    "position_ms": 120000
  }
}
```

### Ejemplo de Respuesta

```json
{
  "status": "ok",
  "playback": {
    "state": "playing",
    "track_id": "a1b2c3d4",
    "position_ms": 120000,
    "duration_ms": 240000,
    "volume": 72,
    "shuffle": false,
    "repeat": "off",
    "device_id": "device-abc-123"
  }
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

La transferencia se solicita mediante:

```json
{
  "command": "transfer_session",
  "params": {
    "target_device_id": "device-xyz-789"
  }
}
```

El dispositivo destino debe aceptar explícitamente la transferencia.

## Modelo de Control Remoto

Un dispositivo móvil puede controlar la reproducción en un servidor o desktop sin necesidad de gestionar el audio localmente.

```
[ Móvil ] --comando--> [ Servidor/Desktop ]
                            |
                        [ Altavoces ]
```

El móvil envía comandos vía HTTP o WebSocket. El servidor ejecuta la reproducción y notifica cambios de estado a todos los clientes suscritos.

## Gestión de Cola

### Estructura de un Elemento de la Cola

| Campo     | Tipo    | Descripción                        |
|-----------|---------|------------------------------------|
| id        | string  | ID único del elemento en la cola   |
| track_id  | string  | ID de la pista                     |
| track     | object  | Datos completos de la pista        |
| added_by  | string  | ID del usuario que la agregó       |
| position  | integer | Posición en la cola                |
| added_at  | string  | Fecha ISO 8601 de agregado         |

### Comandos de Cola

| Comando        | Parámetros                        | Descripción                        |
|----------------|-----------------------------------|------------------------------------|
| queue.add      | `track_id`, `position` (opc.)     | Agregar pista a la cola            |
| queue.remove   | `id`                              | Eliminar elemento de la cola       |
| queue.reorder  | `from_position`, `to_position`    | Reordenar elemento                 |
| queue.jump     | `position`                        | Saltar a posición en la cola       |
| queue.clear    | —                                 | Vaciar la cola                     |

### Ejemplo

```json
{
  "command": "queue.add",
  "params": {
    "track_id": "e5f6g7h8",
    "position": 3
  }
}
```

## Modos de Repetición

| Modo  | Comportamiento                                   |
|-------|--------------------------------------------------|
| off   | No repetir. Al terminar la cola, se detiene.     |
| one   | Repetir la pista actual infinitamente.           |
| all   | Repetir toda la cola al llegar al final.         |
