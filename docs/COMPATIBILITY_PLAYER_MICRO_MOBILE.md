# Guía de Compatibilidad — Player, Micro Server y Mobile

## Situación Actual

Cada proyecto del ecosistema Michi implementa Michi Link a su propio ritmo. Esta guía documenta las diferencias temporales para que los integradores sepan qué esperar al conectar un proyecto con otro.

---

## Diferencias Conocidas

### 1. token/refresh

| Proyecto | Estado |
|----------|--------|
| Michi Music Player | **No implementado.** Player usa sesión persistente sin refresh token. |
| Michi Micro Server | **Implementado.** Micro Server entrega y acepta refresh tokens. |
| Michi Music Mobile | **Consume.** Mobile intenta refresh si el servidor lo declara en `auth.token_refresh`. |

**Regla para Mobile:** Si `server/info` → `auth.token_refresh === false`, no intentes refrescar; el token es persistente. Si `true`, refresca antes de expirar.

### 2. events (WebSocket)

| Proyecto | Estado |
|----------|--------|
| Michi Music Player | **Stub.** Endpoint `/events` declarado pero devuelve 501 o conexión vacía. No hay eventos reales. |
| Michi Micro Server | **Partial.** Micro Server emite broadcast básico de `playback.state_changed` via WebSocket. No implementa otros eventos. |
| Michi Music Mobile | **Planned.** Mobile no consume eventos actualmente. |

### 3. playback/control — command vs action

| Campo | Oficial | Legacy |
|-------|---------|--------|
| `command` | ✅ Sí. Usar siempre. | — |
| `action` | ❌ No. | ✅ Aceptado como fallback. Player y MS aceptan ambos. |
| `position_ms` | ✅ Para seek. | — |
| `position_seconds` | ❌ No. | ✅ Aceptado como fallback. |
| `volume` | ✅ Para set_volume. | — |
| `value` | ❌ No. | ✅ Aceptado como fallback genérico. |

**Regla:** Las apps nuevas DEBEN enviar `command`. Las apps existentes PUEDEN enviar `action` pero deben migrar. Los servidores DEBEN aceptar ambos durante v1.0.x.

### 4. sync/manifest/delta — cursor vs since

| Campo | Oficial | Legacy |
|-------|---------|--------|
| `cursor` | ✅ Sí. String opaco. | — |
| `since` | ❌ No. | ✅ Aceptado como fallback (timestamp ISO 8601). |
| `manifest_id` | ❌ No. | ✅ Aceptado como fallback (ID string). |

**Regla:** Los clientes nuevos DEBEN almacenar y enviar `cursor`. Los servidores DEBEN aceptar `cursor`, `since` y `manifest_id` durante v1.0.x.

### 5. durations y posiciones

| Campo | Oficial | Legacy |
|-------|---------|--------|
| `duration_ms` | ✅ En tracks, queue items. | — |
| `duration_seconds` | ❌ No. | ✅ Aceptado como fallback. |
| `position_ms` | ✅ En playback state. | — |
| `position_seconds` | ❌ No. | ✅ Aceptado como fallback. |

### 6. server/info — formato

| Proyecto | Formato actual |
|----------|----------------|
| Michi Music Player | **Oficial v1** (`service`, `name`, `server_id`, `version`, `api_version`, `michi_link_version`, `roles`, `features`, `auth`) |
| Michi Micro Server | **Oficial v1** (mismos campos) |
| Michi Music Mobile | **Consume.** Mobile no es servidor, no expone /server/info. |

**No se acepta el formato antiguo** (`server_name`, `device_id`, `capabilities`). Clientes que encuentren ese formato deben tratarlo como servidor incompatible.

---

## Matriz de Tolerancia

| Cliente \ Server | Player | Micro Server |
|------------------|--------|-------------|
| **Mobile** | Debe tolerar `token_refresh: false`. No esperar eventos WebSocket. | Debe tolerar `token_refresh: true`. Puede consumir eventos. |
| **Player** (como cliente de MS) | — | Debe enviar `command` (no `action`). Puede recibir `cursor` de vuelta. |
| **Micro Server** (como cliente de Player) | Debe tolerar ausencia de `token/refresh`. | — |

---

## Timeline de Deprecación

| Feature legacy | Deprecado en | Eliminación prevista |
|----------------|-------------|---------------------|
| `action` (playback) | v1.0.0-alpha | v1.1.0 |
| `value` (playback) | v1.0.0-alpha | v1.1.0 |
| `since` / `manifest_id` (sync) | v1.0.0-alpha | v1.1.0 |
| `position_seconds` / `duration_seconds` | v1.0.0-alpha | v1.1.0 |
| `server_name` / `device_id` / `capabilities` | v1.0.0-alpha | v1.0.0 (ya no se acepta) |
