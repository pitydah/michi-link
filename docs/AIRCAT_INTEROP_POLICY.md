# AirCat v1 — Política de Interoperabilidad, Rotación y Errores

Documento normativo complementario para `michi-link`, `michi-micro-server`, `michi-music-stream` y `michi-music-mobile` (`LINK-05`, `LINK-18`, `LINK-20`…`LINK-24`).

---

## 1. Autoridad Home, Rotación y Revocación Irreversible (LINK-05 / LINK-20)

- **Identidad raíz única**: `homeId = base64url_nopad(BLAKE3(root_public_key_32_bytes))` (43 caracteres). Nunca debe confundirse `homeId` con `authorityPublicKeyBase64Url`.
- **Revocación irreversible en autoridad vigente**: Todo miembro revocado queda registrado en `home_revocations` y en memoria; su `session_token` y cualquier `Grant` vigente dejan de ser válidos de inmediato (`403 MEMBER_REVOKED`), incluyendo cancelación en curso durante descargas de `/api/v1/aircat/download/{remoteTrackId}`.
- **Rotación de Home Root**:
  - La rotación de raíz invalida la autoridad anterior y exige re-enrolamiento explícito de miembros (`michi.aircat.home.enrollment.v1`).
  - Prohibido resucitar un `Home` revocado o antiguo mediante un QR vencido (`expiresAtMillis` superado o `homeId` revocado).

## 2. Unicidad de Nonce, TTL y Recuperación tras Reinicio (LINK-18)

- **TTL máximo del Grant**: `<= 600_000 ms` (10 minutos).
- **Antirreplay por `(destinationMichiId, clientNonceBase64Url)`**: El ledger acotado de nonces rechaza cualquier reutilización con `409 NONCE_REPLAY`.
- **Política tras reinicio del productor**: El estado de snapshots y ledger de nonces reside en memoria acotada (`AirCatProducerSecurityState`). Tras un reinicio o caída del productor, los snapshots previos no existen: todo intento de descarga con un grant previo debe rechazarse (`403`/`404`/`409`) y exigir una nueva negociación `manifest -> authorize`.

## 3. Coherencia de Origen ante Cambio de IP / Hostname (LINK-21)

- Los anuncios UDP multicast (`224.0.0.167:53318`) y mDNS (`_michi-link._tcp.local`) son **presencia provisional**, nunca autoridad de transporte.
- Si el servidor productor cambia de IP por DHCP o Tailscale, el cliente Mobile debe rechazar peticiones automáticas hacia la nueva URL B aunque comparta el mismo SPKI hasta que el usuario o flujo administrativo confirme el nuevo `pinnedBaseUrl`.

## 4. Política de Reloj sin SNTP en Michi Music Stream (LINK-22)

- Michi Music Stream (`receiver-lite` ESP32-S3) puede operar en LAN aislada sin acceso a Internet ni servidor SNTP.
- **Prohibido** inventar timestamps o desactivar la verificación de firmas Ed25519 para acelerar el pairing/sesión.
- Si el reloj RTC/SNTP no está sincronizado:
  - El receptor expone el diagnóstico `TIME_NOT_SYNCED`.
  - Los anuncios mDNS permanecen en estado `UNTRUSTED` / provisional hasta completar un desafío criptográfico autenticado (`POST /api/v1/auth/challenge` + `POST /api/v1/auth/session` bajo `HOME_MEMBERSHIP`).

## 5. Catálogo Estable de Códigos de Error AirCat (LINK-23)

Todos los errores AirCat devuelven el envelope canónico `{"error": {"code": "...", "message": "...", "details": {}}}` sin exponer rutas absolutas del sistema de archivos, tokens bearer, claves privadas ni secretos.

| HTTP Status | `error.code` | Semántica |
|---|---|---|
| `401` | `UNAUTHORIZED` / `INVALID_TOKEN` / `MISSING_GRANT` | Sesión ausente, expirada o falta de `X-Michi-AirCat-Grant` |
| `403` | `MEMBER_REVOKED` / `DEVICE_REVOKED` / `UNTRUSTED_MEMBER` / `WRONG_AUDIENCE` / `WRONG_SOURCE` / `BAD_SIGNATURE` / `GRANT_EXPIRED` / `IDENTITY_MISMATCH` | Miembro revocado, audiencia/origen no coincidente, firma inválida o grant caducado |
| `404` | `ASSET_NOT_FOUND` / `TRACK_NOT_FOUND` / `FILE_NOT_FOUND` | Pista inexistente, symlink rechazado o no autorizada en snapshot |
| `409` | `MANIFEST_ALTERED` / `ORIGINAL_BYTES_MISMATCH` / `NONCE_REPLAY` / `FILE_REVISION_CHANGED` | Cambio de bytes/inode/tamaño en disco, hash discrepante o replay de nonce |
| `416` | `RANGE_NOT_SATISFIABLE` | Rango inválido, fuera de límites o multipart no soportado |
| `422` | `INVALID_FIELDS` / `INVALID_GRANT_HEADER` / `INVALID_GRANT_BASE64` / `INVALID_GRANT_JSON` / `INVALID_TRACK_ID` | Payload, manifest vacío/mal formado o cabecera de grant inválida |
| `429` | `RATE_LIMITED` | Límite de concurrencia o tasa superado |

## 6. Matriz de Compatibilidad del Ecosistema (LINK-24)

| Componente | Rol | Protocolo / Auth Vigente | Capacidad AirCat | Estado Pairing Botón / PIN |
|---|---|---|---|---|
| **michi-link** | Autoridad contractual | `v1` / `v1-lite` + `AIRCAT_STRICT` | Normativa + golden vectors | `RECEIVER_BUTTON` marcado `LEGACY_ONLY` |
| **michi-micro-server** | Productor AirCat + Audio Server | `SERVER_CODE` / `HOME_MEMBERSHIP` | `aircat_original_v1` (solo si router+auth+ACL+TLS activos) | Sin botón físico |
| **michi-music-stream** | Receptor físico ESP32-S3 | `HOME_MEMBERSHIP` (`v1-lite`) | `aircat_original=false` (excluido por diseño) | Botón reservado exclusivamente a factory reset |
| **michi-music-mobile** | Consumidor AirCat | `HOME_MEMBERSHIP` / `AIRCAT_STRICT` | Consumidor en `Michi Devices` | Sin fallback cleartext |
| **michi-music-player** | Reproductor escritorio | `PLAYER_PASSWORD` | No aplica | Contraseña local |
