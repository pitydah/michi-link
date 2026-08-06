# Auth Profiles — Michi Link API v1.0.0-alpha

Michi Link define cinco estrategias de autenticación (auth profiles): `PLAYER_PASSWORD`, `SERVER_CODE`, `ED25519_CHALLENGE`, `RECEIVER_BUTTON` y `LEGACY`. Cada proyecto elige la estrategia según su hardware, UX y modelo de seguridad.

**Regla fundamental:** Todo servidor DEBE incluir `auth.required: true` en `/server/info`. Si un servidor no entrega `auth`, debe tratarse como incompatible con v1.0.0-alpha.

---

## PLAYER_PASSWORD

Usado por: **Michi Music Player**

### Flujo

1. Player inicia servidor con una contraseña configurada por el usuario (vía UI de preferencias).
2. El cliente envía `/pair/start` con su identidad y el challenge Ed25519; el servidor abre la sesión y muestra el PIN de 6 dígitos.
3. El usuario ingresa la contraseña en el cliente (autorización humana local) y el PIN en el flujo de confirmación estándar.
4. El servidor valida el PIN y entrega el token Bearer opaco.

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

1. Micro Server inicia y genera un PIN temporal de 6 dígitos (mostrado en su UI web o consola).
2. El cliente envía `/pair/start` con su `device_name`, `device_type`, `roles`, identidad (`michi_id`/`public_key`) y el challenge Ed25519 (`challenge_nonce`/`challenge_signature`).
3. El servidor valida el challenge y asigna una sesión (`session_id`, `expires_at`, `attempts_remaining`) con su identidad (`server_michi_id`/`server_public_key`).
4. El usuario lee el PIN de la pantalla del servidor y lo ingresa en el cliente.
5. El cliente confirma con `/pair/confirm` (`session_id` + `pin`) y recibe token + refresh_token.

### Características

- **Con refresh token:** Micro Server implementa `/token/refresh` completo.
- **PIN en pantalla:** El usuario ve el PIN en el servidor y lo ingresa en el cliente. El PIN nunca viaja por la red.
- **Expiración:** La sesión expira en 5 minutos.
- **Seguridad:** Sesión temporal de un solo uso (5 intentos máximo).

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
3. El receptor envía `/pair/start` (device_type: `receiver`, auth_strategy: `RECEIVER_BUTTON`) al servidor descubierto.
4. El servidor valida el challenge, abre la sesión y muestra el PIN de 6 dígitos en su UI.
5. El usuario confirma el PIN en el servidor (aceptando el nuevo dispositivo) y el receptor completa `/pair/confirm`.

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

## ED25519_CHALLENGE

Usado por: cualquier dispositivo con identidad Ed25519 inicializada (Player, Micro Server, Mobile).

### Flujo

1. Cliente descubre servidor y obtiene su `michi_id` + `public_key` de `GET /api/v1/server/info`.
2. Cliente genera un nonce aleatorio de 16 bytes (base64url, ≥ 22 chars) y lo firma con su secret key (Ed25519 sobre los bytes crudos del nonce).
3. `POST /api/v1/pair/start` con:
   ```json
   {
     "device_name": "Michi Mobile",
     "device_type": "mobile",
     "roles": ["mobile_player", "remote_controller", "sync_client"],
     "auth_strategy": "ED25519_CHALLENGE",
     "michi_id": "<43 chars base64url>",
     "public_key": "<43 chars base64url>",
     "challenge_nonce": ">=22 chars base64url",
     "challenge_signature": "86 chars base64url"
   }
   ```
4. Servidor verifica la firma → prueba de posesión de secret key. Nonce no visto antes (anti-replay).
5. Servidor almacena `public_key` del cliente (TOFU) y abre la sesión con PIN de 6 dígitos mostrado en su UI.
6. Usuario lee el PIN en el servidor y lo ingresa en el cliente.
7. `POST /api/v1/pair/confirm` con `session_id`, `pin`, `michi_id` y `public_key`.
8. Servidor verifica el PIN (verificador keyed, comparación en tiempo constante) → pairing completo, token opaco emitido.

### Características

- **PIN en pantalla:** El PIN es generado por el servidor, mostrado en su UI, y nunca viaja por la red.
- **Sin refresh token:** El token de sesión se obtiene tras confirmar el pairing.
- **TOFU bidireccional:** Ambos lados almacenan la public_key del otro tras el primer challenge exitoso.

### Payload `auth` en `/server/info`

```json
{
  "auth": {
    "required": true,
    "strategy": "ED25519_CHALLENGE",
    "token_refresh": false
  }
}
```

**Nota:** El objeto `auth` canónico solo expone `required`, `strategy` y `token_refresh` (`additionalProperties: false`). La disponibilidad de identidad se expone mediante los campos opcionales `michi_id` y `public_key` de server-info.

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
| Michi Music Mobile | SERVER_CODE (como servidor); como cliente detecta la estrategia del servidor | Según rol | Según rol | No |
| Michi Music Stream (receptor) | RECEIVER_BUTTON | No | No | Sí |

---

## Cómo detecta Mobile la estrategia

1. Mobile consulta `GET /api/v1/server/info` al servidor.
2. Lee `auth.strategy`:
   - `PLAYER_PASSWORD`: Mobile pide contraseña al usuario y completa el flujo canónico de pairing (challenge + sesión + PIN).
   - `SERVER_CODE`: Mobile inicia `/pair/start` con su identidad, muestra el PIN recibido del servidor y espera que el usuario lo ingrese.
   - `RECEIVER_BUTTON`: No aplica (Mobile no se empareja directamente con un receptor).
3. Si `auth.token_refresh` es `false`, Mobile no intentará refrescar el token.
4. Si `auth.token_refresh` es `true`, Mobile usará `/token/refresh` antes de que expire.
