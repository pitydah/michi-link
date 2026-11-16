# Auth Profiles — Michi Link API v1.0.0-alpha

Michi Link define las estrategias de autenticación (auth profiles) del ecosistema: `HOME_MEMBERSHIP` (estrategia primaria de primera clase para Michi Home y receptores modernos), `PLAYER_PASSWORD`, `SERVER_CODE`, `ED25519_CHALLENGE`, `RECEIVER_BUTTON` (LEGACY_ONLY para hardware antiguo) y `LEGACY`. Cada proyecto elige la estrategia según su hardware, UX y modelo de seguridad.

**Regla fundamental:** Todo servidor DEBE incluir `auth.required: true` en `/server/info`. Si un servidor no entrega `auth`, debe tratarse como incompatible con v1.0.0-alpha.

---

## HOME_MEMBERSHIP (Primary Ecosystem Standard)

Usado por: **Michi Music Stream**, **Michi Micro Server**, **Michi Music Mobile** y dispositivos del ecosistema Michi Home Trust V2.

### Identidad y Root Authority
- **Michi ID**: `michi_id = base64url(BLAKE3(public_key_ed25519))` (exactamente 43 caracteres, sin relleno/padding).
- **Home ID**: `home_id = base64url(BLAKE3(home_root_public_key))` (exactamente 43 caracteres, sin relleno/padding).
- La pertenencia a Home se acredita mediante un `DeviceMembershipDto` firmado por la `HomeRootAuthority` persistente.
- Los receptores que no exponen `/api/v1/authority/*` anuncian ausencia de la capability `authority_v1`.

### Flujo de Autenticación
1. **Desafío (`POST /api/v1/auth/challenge`)**:
   El cliente envía `{ "client_michi_id": "<43 chars>" }`.
   El servidor valida que el dispositivo pertenezca al Home, no esté revocado y devuelve `{ "challenge_nonce": "<32 bytes base64url>", "expires_at_ms": ... }`.
2. **Sesión (`POST /api/v1/auth/session`)**:
   El cliente envía:
   - `client_michi_id`: 43 caracteres base64url.
   - `client_membership`: certificado de membresía firmado por `HomeRootAuthority`.
   - `signature`: firma Ed25519 sobre el dominio `MICHI-DEVICE-AUTH-SESSION-V1` + `challenge_nonce`.
   El servidor verifica criptográficamente la membresía y la firma de posesión, comprueba que el dispositivo no esté revocado en `home_revocations`, y emite un `session_token` con lease temporal renovable.
3. **Revocaciones Distribuidas**:
   Cualquier dispositivo revocado en el Home es rechazado de inmediato (403 FORBIDDEN). La revocación es irreversible dentro de la autoridad Home vigente.

### Payload `auth` en `/server/info`

```json
{
  "auth": {
    "required": true,
    "strategy": "HOME_MEMBERSHIP",
    "token_refresh": false
  }
}
```

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

## RECEIVER_BUTTON (LEGACY_ONLY)

> **LEGACY_ONLY**: Esta estrategia está deprecada y se conserva exclusivamente para compatibilidad histórica con firmware antiguo sin aprovisionamiento Home. El firmware actual de Michi Music Stream utiliza exclusivamente `HOME_MEMBERSHIP` vía `/api/v1/auth/challenge` y `/api/v1/auth/session`. El botón físico en hardware actual se reserva únicamente para factory reset o recuperación de provisión Wi-Fi, NUNCA como condición de pairing o identidad en tiempo de ejecución.

Usado por: **Michi Music Stream legado**. Decisiones históricas congeladas en [ADR-0001](adr/ADR-0001-receiver-v1-lite.md).

### Flujo

1. Una pulsación física explícita en el receptor abre una **ventana de 120 segundos**. Reiniciar el dispositivo cierra la ventana; abrir de nuevo reemplaza la ventana previa y elimina las sesiones de pairing pendientes.
2. El cliente controlador (Micro Server, `device_type: "server"`, `roles: ["music_server"]`, `auth_strategy: "RECEIVER_BUTTON"`) envía `/pair/start` con su identidad y el challenge Ed25519. Fuera de la ventana física, `/pair/start` responde `403 FORBIDDEN`.
3. El receptor valida el challenge, comprueba que `michi_id` corresponde a `public_key` y crea la sesión. Genera un PIN criptográficamente aleatorio de seis dígitos, lo muestra localmente y **nunca lo devuelve por HTTP**.
4. Bajo el modelo de red LAN de confianza, el cliente envía el PIN en `/pair/confirm`. Máximo cinco intentos fallidos por sesión; después, `429 RATE_LIMITED` y la sesión queda consumida.
5. El receptor emite el token Bearer: 32 bytes de CSPRNG codificados en base64url sin padding, devuelto una sola vez. El receptor persiste únicamente el SHA-256 del token. `expires_in: 0` significa "sin expiración automática; válido hasta revocación o factory reset".
6. La sesión se consume tras el éxito; una segunda confirmación responde `409 CONFLICT`.

### Características

- **Sin refresh token:** `token_refresh: false`. El token lo emite el receptor y no expira solo.
- **Sin UI compleja:** El receptor solo tiene botón físico y LEDs de estado.
- **Heartbeat:** La sesión se mantiene activa vía heartbeat cada 10s con lease de 30 segundos.
- **Seguridad:** Basada en proximidad física (botón) y en el modelo LAN de confianza.

### Payload `auth` en `/server/info`

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
| Michi Music Stream (receptor actual) | HOME_MEMBERSHIP | No | No | No (solo reset) |
| Michi Micro Server | HOME_MEMBERSHIP / SERVER_CODE | Sí | Sí (web/console) | No |
| Michi Music Mobile | HOME_MEMBERSHIP / SERVER_CODE | Según rol | Según rol | No |
| Michi Music Player | PLAYER_PASSWORD | No | No (contraseña en UI) | No |
| Hardware legado (firmware previo) | RECEIVER_BUTTON (LEGACY_ONLY) | No | No | Sí |

---

## Cómo detecta Mobile la estrategia

1. Mobile consulta `GET /api/v1/server/info` al servidor.
2. Lee `auth.strategy`:
   - `PLAYER_PASSWORD`: Mobile pide contraseña al usuario y completa el flujo canónico de pairing (challenge + sesión + PIN).
   - `SERVER_CODE`: Mobile inicia `/pair/start` con su identidad, muestra el PIN recibido del servidor y espera que el usuario lo ingrese.
   - `RECEIVER_BUTTON`: No aplica (Mobile no se empareja directamente con un receptor).
3. Si `auth.token_refresh` es `false`, Mobile no intentará refrescar el token.
4. Si `auth.token_refresh` es `true`, Mobile usará `/token/refresh` antes de que expire.

---

## AIRCAT_STRICT (Original Audio Transfer Transport Profile)

Perfil obligatorio de transporte para la transferencia de archivos originales AirCat v1 (`michi.aircat.original.v1`) entre `Michi Micro Server` (productor) y `Michi Music Mobile` (consumidor).

### Requisitos de Transporte (LINK-06 / LINK-15 / LINK-16)

1. **HTTPS Obligatorio**: Todas las rutas `/api/v1/aircat/*` (`/manifest`, `/authorize`, `/download/{remoteTrackId}`, `/enrollment-qr`) deben cursarse sobre HTTPS con validación de hostname/CA y **pin SPKI RFC 7469** (`sha256/<BASE64_STANDARD>` con padding `=`) derivado del certificado TLS X.509 real del servidor.
2. **Prohibición de Downgrade y Redirecciones**:
   - Prohibido el fallback silencioso a HTTP cleartext.
   - Prohibido `trustAll`, `HostnameVerifier { true }` o aceptar certificados/pins provenientes de anuncios mDNS/UDP no autenticados.
   - Prohibido seguir redirecciones HTTP (`3xx`) en clientes AirCat.
3. **Ciclo de Vida del Pin SPKI y Migración de URL/IP (LINK-21)**:
   - El pin `tlsSpkiSha256` y la URL origen `pinnedBaseUrl` se establecen únicamente mediante enrolamiento administrativo explícito (`michi.aircat.home.enrollment.v1` vía QR autenticado).
   - Un cambio de IP DHCP o hostname en anuncios Discovery/mDNS **no autoriza** migrar silenciosamente `pinnedBaseUrl` ni sustituir el pin SPKI; requiere reenrolamiento explícito confirmado.
4. **Cabeceras Wire Obligatorias (`michi.aircat.original.v1`)**:
   - Todas las respuestas exitosas (`200`, `206`, `416` aplicable): `X-Michi-AirCat-Protocol: michi.aircat.original.v1` (valor único, sin duplicados).
   - Respuestas JSON (`/manifest`, `/authorize`, `/enrollment-qr`): `Content-Type: application/json` y `Cache-Control: no-store`.
   - Descargas binarias (`/download/{remoteTrackId}`): `Content-Type: application/octet-stream`, `Cache-Control: no-transform`, `Accept-Ranges: bytes`, `ETag: "sha256-<hex>-<size>"` fuerte, `X-Michi-AirCat-Authorized-Grant-SHA256: <hex>`, y rechazo de compresión (`Accept-Encoding: identity`).
