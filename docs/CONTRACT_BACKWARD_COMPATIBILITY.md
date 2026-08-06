# Retrocompatibilidad del Contrato Michi Link v1

## Principio Fundamental

`api_version: "v1"` es **permanente**. Mientras un dispositivo declare `api_version: "v1"`, debe ser compatible con cualquier otro dispositivo que también declare `"v1"`, independientemente de las versiones de app (`version`) o de las capacidades individuales. La versión del contrato NO cambia con versiones de app.

## Reglas de Retrocompatibilidad

1. **Campos nuevos son siempre opcionales.** Ningún schema existente cambia campos `required`.
2. **Nuevos valores en enum no rompen parsing.** Los clientes ignoran valores desconocidos.
3. **Announces sin firma se aceptan siempre.** La firma es un campo adicional.
4. **UUID `device_id` se mantiene para siempre en v1.** `michi_id` es adicional.
5. **`michi_id` se omite** cuando el servidor no tiene identidad inicializada (campo opcional; no se envía `null`).

## Tabla de Cambios

| Feature | Versión añadida | ¿Rompe v1? | Mecanismo de retrocompatibilidad |
|---------|----------------|-----------|----------------------------------|
| `michi_id` en server-info | 1.0.0-beta | ❌ No | Campo opcional. Servidores sin identidad lo omiten. Clientes legacy ignoran el campo. |
| `public_key` en server-info | 1.0.0-beta | ❌ No | Campo opcional. Omitido si no hay identidad. |
| Announces firmados (mDNS/UDP) | 1.0.0-beta | ❌ No | Firma en campo nuevo (`signature`). Ausente = legacy. Firmas inválidas → se descartan. |
| `auth_strategy: ED25519_CHALLENGE` | 1.0.0-beta | ❌ No | Nuevo valor en enum. Si el cliente no lo entiende, usa legacy. |
| QR pairing | 1.0.0-beta | ❌ No | Canal nuevo fuera de banda. No afecta REST API. |
| TOFU storage | 1.0.0-beta | ❌ No | Solo lectura interna del servidor. No afecta endpoints. |
| `pin_proof` en pair/confirm | 1.0.0-beta | ❌ No | Campo opcional. Pairing legacy funciona sin él. |
| `device_id` (UUID) | 1.0.0-alpha | ❌ Nunca | Se mantiene para siempre en v1. |
| `action` legacy | 1.0.0-alpha | ❌ No | Deprecado pero aceptado como alias de `command`. |
| `since` / `manifest_id` | 1.0.0-alpha | ❌ No | Deprecados pero aceptados como alias de `cursor`. |

## Glosario de Retrocompatibilidad

| Término | Significado |
|---------|-------------|
| **Aditivo** | El cambio solo agrega campos nuevos. No modifica, elimina ni cambia tipo de campos existentes. |
| **Opcional** | El campo no está en `required`. Si no está presente, el comportamiento es el mismo que antes. |
| **Legacy** | Mecanismo anterior que sigue siendo aceptado pero no se recomienda para nuevas implementaciones. |
| **Fallback** | Mecanismo alternativo que se activa cuando el mecanismo principal no está disponible. |
| **TOFU** | Trust On First Use. La primera vez que se ve una clave pública, se confía en ella. |

## Escenarios de Compatibilidad

### Cliente v1.0.0-alpha ↔ Servidor v1.0.0-beta
- Cliente no conoce `michi_id`, `public_key`. Los ignora.
- Cliente no conoce `ED25519_CHALLENGE`. Usa `SERVER_CODE` o `PLAYER_PASSWORD`.
- Cliente no puede verificar announces firmados. Los trata como `untrusted`.
- **Todo funciona exactamente igual que antes.**

### Cliente v1.0.0-beta ↔ Servidor v1.0.0-alpha
- Cliente busca `michi_id` en server-info → no está (omitido). Sabe que el servidor no tiene identidad.
- Cliente recibe announce sin firma → lo acepta como `untrusted`.
- **Todo funciona exactamente igual que antes.**

### Cliente v1.0.0-beta (con identidad) ↔ Servidor v1.0.0-beta (con identidad)
- Ambos tienen `michi_id`. Pueden verificar announces mutuamente.
- Pueden usar `ED25519_CHALLENGE` para pairing sin PIN.
- QR pairing disponible si ambos tienen cámara/pantalla.
- **Máximo nivel de seguridad y confianza.**

## Versionado

`api_version` es la única versión del contrato:

- `api_version: "v1"` — contrato completo. Permanente: un dispositivo "v1" debe ser compatible con cualquier otro "v1".
- `api_version: "v1-lite"` — subconjunto para receivers (codecs pcm_s16le/pcm_s24le, auth RECEIVER_BUTTON, heartbeat 10s; sin library/playlists/search/sync/storage/transcoding/rooms).

La versión del contrato NO cambia con versiones de app (`version` en server-info es la versión de la aplicación, no del contrato). `michi_link_version` fue retirado en v1 y no es parte del contrato. <!-- michi-policy:exclude -->
