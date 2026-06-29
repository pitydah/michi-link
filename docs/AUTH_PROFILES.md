# Auth Profiles — Michi Link API v1.0.0-alpha

Michi Link define cuatro estrategias de autenticación (auth profiles). Cada proyecto elige la estrategia según su hardware, UX y modelo de seguridad.

---

## PLAYER_PASSWORD

Usado por: **Michi Music Player**

### Flujo

1. Player inicia servidor con una contraseña configurada por el usuario (vía UI de preferencias).
2. El cliente (Mobile, otro Player) envía en `/pair/start` la contraseña como prueba.
3. El servidor valida la contraseña y genera un `pairing_code` y `device_id`.
4. El cliente confirma con `/pair/confirm` y recibe un token Bearer.

### Características

- **Sin refresh token:** Player no implementa `/token/refresh`. El token es de larga duración (sesión persistente).
- **Sin código en pantalla:** El pairing requiere que el usuario ingresó la contraseña manualmente en el cliente.
- **Seguridad:** Contraseña configurable, visible solo en la UI del Player.

### Payload `auth` en `/server/info`

```json
{
  "auth": {
    "required": true,
    "strategy": "PLAYER_PASSWORD",
    "token_refresh": false
  }
}
```

---

## SERVER_CODE

Usado por: **Michi Micro Server**

### Flujo

1. Micro Server inicia y genera un código de pairing temporal (6-8 caracteres alfanuméricos).
2. El código se muestra en una UI web o consola del servidor.
3. El cliente envía `/pair/start` con su `device_name` y `device_type`.
4. El servidor asigna un `pairing_code` y `device_id`.
5. El cliente muestra el código al usuario, quien lo ingresa en el servidor.
6. El servidor confirma con `/pair/confirm` y entrega token + refresh_token.

### Características

- **Con refresh token:** Micro Server implementa `/token/refresh` completo.
- **Código en pantalla:** El usuario ve el código en el servidor y lo ingresa en el cliente (o viceversa).
- **Expiración:** Código expira en 5 minutos.
- **Seguridad:** Código temporal de un solo uso.

### Payload `auth` en `/server/info`

```json
{
  "auth": {
    "required": true,
    "strategy": "SERVER_CODE",
    "token_refresh": true
  }
}
```

---

## RECEIVER_BUTTON

Usado por: **Michi Music Stream** (Standard y Hi-Fi)

### Flujo

1. El receptor (Stream) se enciende y busca servidores vía UDP/mDNS.
2. El usuario presiona un botón físico en el receptor para iniciar pairing.
3. El receptor envía `/receiver/pair/start` al servidor descubierto.
4. El servidor genera un código y lo muestra en su UI.
5. El usuario confirma el código en el servidor (aceptando el nuevo dispositivo).
6. Alternativa: pairing automático por botón (sin código) si solo hay un servidor en la red.

### Características

- **Sin refresh token:** El receptor usa un token interno fijo mientras esté emparejado.
- **Sin UI compleja:** El receptor solo tiene botón físico y LEDs de estado.
- **Heartbeat:** El receptor mantiene sesión activa vía heartbeat cada 10s.
- **Seguridad:** Basada en proximidad física (botón).

### Payload `auth` en `/receiver/info`

```json
{
  "auth": {
    "required": true,
    "strategy": "RECEIVER_BUTTON",
    "token_refresh": false
  }
}
```

---

## LEGACY

Usado como transición por clientes que aún implementan `action`/`value`.

### Características

- Acepta campos antiguos (`action`, `value`, `since`, `manifest_id`, `position_seconds`, `duration_seconds`).
- No recomendado para nuevas implementaciones.
- Se eliminará en v1.1.0 o v2.0.0.

---

## Tabla de Estrategias por Proyecto

| Proyecto | Estrategia | token_refresh | Código en pantalla | Botón físico |
|----------|-----------|---------------|-------------------|--------------|
| Michi Music Player | PLAYER_PASSWORD | No | No (contraseña en UI) | No |
| Michi Micro Server | SERVER_CODE | Sí | Sí (web/console) | No |
| Michi Music Mobile | Cliente que detecta estrategia | Según servidor | Según servidor | No |
| Michi Music Stream (receptor) | RECEIVER_BUTTON | No | No | Sí |

---

## Cómo detecta Mobile la estrategia

1. Mobile consulta `GET /api/v1/server/info` al servidor.
2. Lee `auth.strategy`:
   - `PLAYER_PASSWORD`: Mobile pide contraseña al usuario y la envía en `/pair/start`.
   - `SERVER_CODE`: Mobile muestra el código recibido y espera confirmación del servidor.
   - `RECEIVER_BUTTON`: No aplica (Mobile no se empareja directamente con un receptor).
3. Si `auth.token_refresh` es `false`, Mobile no intentará refrescar el token.
4. Si `auth.token_refresh` es `true`, Mobile usará `/token/refresh` antes de que expire.
