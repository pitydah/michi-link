# Michi Identity — Sistema de Identidad Descentralizada

## Introducción

Michi Identity es un sistema de identidad y confianza descentralizado basado en criptografía Ed25519. Permite que los dispositivos del ecosistema Michi se reconozcan por su **identidad única** (derivada criptográficamente de su clave pública) en lugar de depender de IPs, UUIDs volátiles o secretos compartidos.

## Componentes

### 1. IdentityManager

Genera y persiste un par de claves Ed25519 por dispositivo. Deriva un `michi_id` como `blake3(public_key)`.

**Persistencia:** `~/.config/michi/identity.msgpack` (MessagePack). La clave secreta se guarda cifrada con XOR + blake3(hostname + salt) para evitar que una copia del archivo sea usable en otra máquina.

**Métodos principales:**
- `load_or_generate(config_dir, device_name)` — carga existente o genera nuevo par
- `sign_standard(payload)` — firma y retorna (signature_b64, public_key_b64)
- `sign_raw(payload)` — firma y retorna (signature_bytes, public_key_bytes)
- `verify(payload, sig_b64, pk_b64)` — verificación estática (no necesita instancia)
- `derive_michi_id(pk_b64)` — deriva MichiId desde public key
- `michi_id()` — retorna el MichiId propio
- `public_key_b64()` / `public_key_bytes()` — retorna la clave pública

### 2. DiscoveryEngine

Anuncia presencia en la red con announces **firmados** opcionalmente.

**Flujo:**
1. `build_signed_announce()` — construye payload canónico y lo firma con Ed25519.
2. `verify_announce(announce)` — verifica firma. Retorna `Verified(michi_id)`, `Untrusted(uuid)` o `Invalid`.
3. Announces **sin firma** se aceptan como `Untrusted` (retrocompatible).
4. Announces **con firma inválida** se rechazan.

### 3. PairingProtocol

Emparejamiento TOFU (Trust On First Use) con PIN de 6 dígitos y verificación de firma Ed25519.

**Flujo:**

```
Cliente                              Servidor
  │  POST /pair/start                   │
  │  { public_key                       │
  │    challenge_nonce                  │
  │    challenge_signature }            │
  │ ──────────────────────────────────> │
  │                                     │  Verifica firma del challenge
  │                                     │  Genera PIN de 6 dígitos
  │  { server_pk, server_nonce,         │
  │    server_signature, pin_hash }     │
  │ <────────────────────────────────── │
  │                                     │
  │  POST /pair/confirm                 │
  │  { pin, pin_proof,                  │
  │    pin_proof_signature }            │
  │ ──────────────────────────────────> │
  │                                     │  Verifica pin_proof_signature
  │                                     │  Verifica pin_proof contra hash
  │                                     │  TOFU completo
```

### 4. QRConnector

Genera y parsea URIs firmadas para pairing fuera de banda (código QR).

**Formato URI:**
```
michi://pair?pk=<base64url>&id=<hex>&nonce=<base64url>&sig=<base64url>&name=<url>
```

**Flujo:**
1. Mobile genera URI firmada con su clave.
2. Mobile muestra QR.
3. Player/Micro escanea → verifica firma → obtiene pk del Mobile.
4. Usa esa pk para iniciar pairing TOFU sin necesidad de discovery.

## Integración en el Ecosistema

### Inicialización en cualquier app Michi

```rust
use michi_identity::init;

#[tokio::main]
async fn main() {
    let config_dir = std::path::Path::new("/home/user/.config/michi");
    let (identity, discovery) = init(config_dir, "Mi Dispositivo").await.unwrap();
    println!("Michi ID: {}", identity.michi_id());
}
```

### En server-info

```json
{
  "michi_id": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
  "public_key": "base64_encoded_public_key"
}
```

Ambos campos son **opcionales**. Si no hay identidad inicializada, se envía `null`.

### Auth Strategy

Cuando un servidor tiene identidad Ed25519, puede anunciar:

```json
{
  "auth": {
    "required": true,
    "strategy": "SERVER_CODE",
    "identity_available": true,
    "identity_strategies": ["ED25519_CHALLENGE"]
  }
}
```

Un cliente compatible puede elegir `ED25519_CHALLENGE` para pairing sin PIN.

## Retrocompatibilidad

| Cambio | ¿Rompe v1? | Mecanismo |
|--------|-----------|-----------|
| `michi_id` en server-info | ❌ No | Campo opcional. `null` = legacy |
| `public_key` en server-info | ❌ No | Campo opcional. `null` = legacy |
| Announces firmados | ❌ No | Firma en campo nuevo. Ausente = legacy |
| `auth_strategy: ED25519_CHALLENGE` | ❌ No | Nuevo valor en enum. Ausente = legacy |
| QR pairing | ❌ No | Canal nuevo. No reemplaza nada |
| TOFU storage | ❌ No | Solo lectura. No afecta endpoints |
| UUID `device_id` | ❌ Nunca | Se mantiene para siempre en v1 |
