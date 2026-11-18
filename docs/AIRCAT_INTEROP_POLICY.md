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

---

## 7. Canonización Binaria Normativa (`LINK-03` / `LINK-17`)

AirCat v1 **no** firma serializaciones JSON RFC 8785 implícitas ni órdenes de claves variables; firma bytes binarios deterministas definidos en `crates/michi-identity/src/aircat.rs`:

- **Domain separators**: `"MICHI-AIRCAT-MANIFEST-V1"` y `"MICHI-AIRCAT-GRANT-V1"`.
- **Codificación de enteros**: `i32` (`4` bytes) y `i64` (`8` bytes) en orden **big-endian**.
- **Codificación de cadenas UTF-8** (`write_field`): prefijo `u32` big-endian con la longitud en bytes (máximo `4096` bytes por campo) seguido por los bytes UTF-8 sin padding.
- **Orden estricto del manifiesto** (`MICHI-AIRCAT-MANIFEST-V1`):
  `domain -> manifestVersion (i32=1) -> transferId -> sourceDeviceId -> totalTracks (i32) -> totalBytes (i64) -> itemCount (i32)` seguido por cada track único en orden exacto de la selección:
  `logicalTrackId -> remoteTrackId -> title -> artist -> album -> format -> size (i64) -> sha256_lowercase`.
- **Orden estricto del grant** (`MICHI-AIRCAT-GRANT-V1`):
  `domain -> version (i32=1) -> homeId -> authorityMichiId -> transferId -> sourceMichiId -> destinationMichiId -> sourceLibraryId -> destinationLibraryId -> selectionScope -> scopeCount (i32=2) -> "content:read" -> "content:transfer" -> manifestSha256 -> issuedAtMillis (i64) -> expiresAtMillis (i64) -> nonceBase64Url`.
- **Golden vectors canónicos** (`tests/vectors/aircat/aircat-golden-v1.*` y `fixtures/aircat/`):
  - Grant canónico binario: `451` bytes (`signatureBase64Url = "ccQtsU8Nzu4zQAQ_WZf_oFCTd07QJJptMDckzbeay1FmgF1oJuAYM_TWgIAXVyOe3kApsJqIoJPQqLa5iIwKBA"`).
  - Manifiesto canónico de prueba: `df2221e0e394146f646543f4c824f00c7c3b40e1950d7a42d9db57f84597b066`.
  - Recibo HTTP: `hex(sha256(ASCII(grant_header_raw))) = a9b0ac283d6da5511cc935fab32d1e739f868b2aaf82568e8f8f08a701c6b300`.
- **Tolerancia cero**: cualquier campo adicional no firmado, Scope distinto de `["content:read", "content:transfer"]` o discrepancia de orden invalida la firma Ed25519 antes de leer bytes de audio.

---

## 8. Contratos Complementarios del Ecosistema (Sección `10.6`)

| Flujo | Fuente de autoridad | Invariante de interoperabilidad y no interferencia |
|---|---|---|
| `Michi Control` | Autoridad de sesión de audio del propietario activo | El handoff de reproducción entre `michi-music-player`, `michi-music-mobile` y `michi-micro-server` no muta `homeId`, membresía ni posesión de grants AirCat. |
| `NowPlaying` | `AudioEngine` / `Media3` o `PlaybackEngine` de `michi-micro-server` | Refleja estado de reproducción en tiempo real sin interferir con el plano de descarga original ni simular progreso de copia AirCat. |
| `RTP Multiroom` | Sesión de salas en `michi-micro-server` y `michi-music-stream` (`receiver-v1-lite`) | Audio PCM S16LE 48 kHz/2ch PT97 en bloques 10 ms (`1920` bytes) con SSRC fijo; las transferencias AirCat usan semáforos y buffers de 64 KiB fuera del hilo RTP. |
| `Scent / Whisker` | Anuncios UDP multicast / mDNS firmados (`_michi-link._tcp.local`) | Proveen únicamente presencia provisional (`±90 s`); nunca elevan confianza ni autorizan cambio silencioso de URL TLS pinneada. |
| `Library metadata` | Biblioteca original en productor + Room local tras importación | Mantiene `remoteTrackId != logicalTrackId` cuando aplique; metadatos, carátulas o etiquetas no sustituyen ni alteran el SHA-256 del archivo binario original. |
| `Kitty AI` (futuro) | Motor on-demand local en `michi-music-mobile` | Puede diagnosticar transferencias usando recibos y códigos saneados; prohibido leer secretos Home, tokens Bearer o iniciar discovery autónomo. |
| `Devices USB / MTP` | Almacenamiento conectado + Storage Access Framework (SAF) | Separado por feature-gate independiente respecto a AirCat LAN (`michi.aircat.original.v1`); no asume soporte USB accessory activo sobre transporte HTTP. |

