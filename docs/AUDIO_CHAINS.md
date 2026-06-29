# Arquitectura de Cadenas de Audio

## Concepto

Una **cadena de audio** (audio chain) es el conducto completo que lleva el audio desde su origen hasta su destino. Se compone de cuatro elementos:

```
Fuente → Controlador → Salida → Perfil
```

| Componente   | Descripción                                         |
|--------------|-----------------------------------------------------|
| **Fuente** (Source)    | De dónde proviene el audio                    |
| **Controlador** (Controller) | Pipeline de procesamiento de audio       |
| **Salida** (Output)    | Hacia dónde se envía el audio                |
| **Perfil** (Profile)   | Preset con nombre que combina los tres anteriores |

## Fuente (Source)

| Tipo     | Descripción                              | Config                               |
|----------|------------------------------------------|--------------------------------------|
| local    | Archivo de audio en el dispositivo       | `path`, `format`                     |
| stream   | URL de streaming (HLS, Icecast, etc.)    | `url`, `protocol`, `headers`         |
| radio    | Emisora de radio por streaming           | `station_id`, `url`, `name`          |
| playlist | Lista de reproducción de la biblioteca   | `playlist_id`, `shuffle`, `mode`     |

## Controlador (Controller)

| Tipo        | Descripción                                          | Config                                |
|-------------|------------------------------------------------------|---------------------------------------|
| builtin     | Pipeline por defecto sin procesamiento extra         | `{}`                                  |
| equalizer   | Equalizador gráfico de bandas                        | `bands`, `preamplifier`               |
| crossfade   | Transición suave entre pistas                        | `duration_ms`, `overlap`              |
| normalize   | Normalización de volumen (EBU R128 / ReplayGain)     | `target_lufs`, `gating`               |

## Salida (Output)

| Tipo      | Descripción                                           | Config                                  |
|-----------|-------------------------------------------------------|-----------------------------------------|
| local     | Altavoces del dispositivo                             | `device`, `channels`                    |
| receiver  | Receptor físico v1-lite                               | `receiver_id`, `protocol`               |
| room      | Zona/room del sistema                                 | `room_id`                               |
| multiroom | Múltiples salidas sincronizadas                       | `outputs: [{type, config}]`             |

## Perfil (Profile)

Los perfiles permiten guardar y reutilizar configuraciones completas.

| Perfil       | Descripción                         |
|--------------|-------------------------------------|
| default      | Configuración por defecto           |
| night        | Volumen reducido, sin graves        |
| party        | Graves potenciados, volumen alto    |
| headphones   | Ecualización para auriculares       |
| custom       | Perfil definido por el usuario      |

## Esquema de Configuración

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Cadena Principal",
  "source": {
    "type": "local",
    "config": {
      "path": "/music/albums/",
      "format": "flac"
    }
  },
  "controller": {
    "type": "equalizer",
    "config": {
      "bands": [
        {"freq": 60, "gain": 0},
        {"freq": 230, "gain": 2},
        {"freq": 910, "gain": 1},
        {"freq": 4000, "gain": -1},
        {"freq": 14000, "gain": 0}
      ],
      "preamplifier": 0
    }
  },
  "output": {
    "type": "room",
    "config": {
      "room_id": "sala-estar"
    }
  },
  "profile": "party"
}
```

## Roles y Uso

| Rol       | Uso típico                                          |
|-----------|------------------------------------------------------|
| Usuario   | Selecciona perfil, ajusta ecualizador, cambia salida |
| Admin     | Configura cadenas para zonas comunes                 |
| Sistema   | Crea cadena automática al reproducir en una room     |

## Multiroom

Una misma fuente puede enviarse a múltiples salidas sincronizadas:

```json
{
  "id": "multiroom-chain-001",
  "name": "Toda la Casa",
  "source": {
    "type": "stream",
    "config": {
      "url": "https://stream.example.com/musica"
    }
  },
  "controller": {
    "type": "builtin",
    "config": {}
  },
  "output": {
    "type": "multiroom",
    "config": {
      "outputs": [
        {"type": "room", "config": {"room_id": "sala"}},
        {"type": "room", "config": {"room_id": "cocina"}},
        {"type": "receiver", "config": {"receiver_id": "rec-hifi-01"}}
      ],
      "sync_method": "master_clock"
    }
  },
  "profile": "default"
}
```

## Ejemplo Completo

```json
{
  "id": "chain-party-001",
  "name": "Fiesta Jardín",
  "source": {
    "type": "playlist",
    "config": {
      "playlist_id": "pl-fiesta",
      "shuffle": true,
      "mode": "all"
    }
  },
  "controller": {
    "type": "crossfade",
    "config": {
      "duration_ms": 3000,
      "overlap": 0.5
    }
  },
  "output": {
    "type": "receiver",
    "config": {
      "receiver_id": "rec-jardin",
      "protocol": "v1-lite"
    }
  },
  "profile": "party"
}
```
