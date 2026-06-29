# Gestión de Zonas (Rooms)

## Modelo de Room

Una **room** representa una zona lógica de reproducción. Puede agrupar múltiples dispositivos y receivers.

| Campo         | Tipo    | Descripción                                 |
|---------------|---------|---------------------------------------------|
| id            | string  | Identificador único de la room              |
| name          | string  | Nombre legible (ej. "Sala", "Cocina")       |
| devices       | array   | IDs de dispositivos lógicos asociados       |
| receivers     | array   | IDs de receivers físicos asociados          |
| active_chain  | string  | ID de la cadena de audio activa             |
| volume        | integer | Volumen 0–100                               |
| is_playing    | boolean | Si hay reproducción activa en la room       |
| current_track | object  | Pista actual (opcional, si está sonando)    |

### Ejemplo

```json
{
  "id": "sala-estar",
  "name": "Sala de Estar",
  "devices": ["device-android-papa", "device-iphone-mama"],
  "receivers": ["rec-hifi-001"],
  "active_chain": "chain-party-001",
  "volume": 65,
  "is_playing": true,
  "current_track": {
    "id": "a1b2c3d4",
    "title": "Bohemian Rhapsody",
    "artist": "Queen",
    "album": "A Night at the Opera"
  }
}
```

## Multiroom

El sistema multiroom permite reproducir el mismo audio en múltiples rooms sincronizadas.

```
[ Servidor ]
     |
     ├── Room "Sala"     → Receiver Hi-Fi
     ├── Room "Cocina"   → Receiver Standard
     └── Room "Jardín"   → Receiver Standard
```

La sincronización se logra mediante un reloj maestro compartido. Todas las salidas reciben el mismo flujo de audio con un offset calculado para compensar latencia de red.

## Gestión de Rooms

### Crear Room

`POST /api/v1/rooms`

```json
{
  "name": "Oficina",
  "receivers": ["rec-oficina-01"]
}
```

### Listar Rooms

`GET /api/v1/rooms`

### Obtener Room

`GET /api/v1/rooms/{id}`

### Actualizar Room

`PUT /api/v1/rooms/{id}`

```json
{
  "name": "Oficina Principal",
  "receivers": ["rec-oficina-01", "rec-oficina-02"]
}
```

### Eliminar Room

`DELETE /api/v1/rooms/{id}`

## Reproducir en una Room

Reproducir en una room crea automáticamente una cadena de audio con la room como salida.

`POST /api/v1/rooms/{id}/play`

```json
{
  "track_id": "a1b2c3d4",
  "source": {
    "type": "local",
    "config": {}
  },
  "controller": {
    "type": "builtin",
    "config": {}
  }
}
```

**Respuesta:**

```json
{
  "status": "playing",
  "chain_id": "chain-auto-001",
  "room": {
    "id": "sala-estar",
    "name": "Sala de Estar",
    "is_playing": true,
    "volume": 65
  }
}
```

## Agrupación de Rooms

Para reproducción sincronizada, las rooms se agrupan en un **grupo multiroom**.

`POST /api/v1/rooms/groups`

```json
{
  "name": "Toda la Casa",
  "room_ids": ["sala", "cocina", "jardin"],
  "sync_method": "master_clock"
}
```

### Ejemplo de Grupo

```json
{
  "id": "group-casa",
  "name": "Toda la Casa",
  "rooms": ["sala", "cocina", "jardin"],
  "active_chain": "chain-multiroom-001",
  "is_playing": true,
  "sync_method": "master_clock",
  "master_room": "sala"
}
```
