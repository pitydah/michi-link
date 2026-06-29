# Queue Transfer — Michi Link API v1.0.0-alpha

## Propósito

Transferir la cola de reproducción actual del Player al Micro Server como parte del flujo Continue-on-Server.

## Endpoint: POST /api/v1/queue/transfer

**Auth:** Bearer token con permiso `queue.write`.

### Request

```json
{
  "track_ids": [
    "server_track_uuid_abc",
    "server_track_uuid_def",
    "server_track_uuid_ghi"
  ],
  "current_index": 1,
  "position_ms": 45000,
  "source": "michi-music-player",
  "queue_id": "player_queue_uuid"
}
```

### Response

```json
{
  "queue_id": "micro_queue_uuid",
  "accepted": true,
  "items_count": 3,
  "current_index": 1,
  "position_ms": 45000
}
```

### Campos

#### Request

| Campo | Tipo | Obligatorio | Descripción |
|-------|------|-------------|-------------|
| `track_ids` | string[] | Sí | Lista ordenada de `remote_track_id` a reproducir. |
| `current_index` | int | Sí | Índice actual en la cola (0-indexed). |
| `position_ms` | int | Sí | Posición de reproducción en milisegundos. |
| `source` | string | Sí | Identificador del dispositivo origen. |
| `queue_id` | string | No | ID de la cola en el origen para trazabilidad. |

#### Response

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `queue_id` | string | ID de la cola creada en el servidor destino. |
| `accepted` | bool | `true` si la transferencia fue aceptada. |
| `items_count` | int | Número de tracks en la cola remota. |
| `current_index` | int | Índice actual confirmado. |
| `position_ms` | int | Posición confirmada. |

### Diferencia con POST /queue/items

| Aspecto | `POST /queue/items` | `POST /queue/transfer` |
|---------|---------------------|------------------------|
| Alcance | Agregar tracks a cola existente | Reemplazar cola completa |
| current_index | No acepta | Sí, posición actual |
| position_ms | No acepta | Sí, tiempo de reproducción |
| source | No requiere | Sí, para trazabilidad |

### Integración con Continue-on-Server

Queue transfer es el paso 4 del flujo de Continue-on-Server. Después de:

1. ✅ Preflight
2. ✅ Upload tracks faltantes
3. ✅ Commit mapping

Se ejecuta:

4. **Queue transfer** (`POST /api/v1/queue/transfer`)
5. Iniciar reproducción (`POST /api/v1/playback/control { command: "play" }`)
6. Confirmar estado (`GET /api/v1/playback/state`)
