# Michi Link E2E Certification

## Cómo ejecutar escenarios E2E

### Prerrequisitos

```bash
pip install requests pyyaml
```

Todos los escenarios se ejecutan con `runner.py`:

```bash
cd tests/e2e_certification
python runner.py scenarios/<scenario>.yml --server-host <IP> --server-port <PORT>
```

### Escenario A: Mobile ↔ Player

**Requisitos:**
- Michi Music Player corriendo en la red local (ej: 192.168.1.100:8400).
- Mobile test client (o curl manual si no hay client).

**Ejecución:**

```bash
# Pairing
python runner.py scenarios/mobile_player.yml --server-host 192.168.1.100 --server-port 8400 --certified-by dev@example.com

# Playback control (requiere token del pairing anterior)
python runner.py scenarios/mobile_player.yml --server-host 192.168.1.100 --server-port 8400 --token <TOKEN>
```

### Escenario B: Mobile ↔ Micro Server

**Requisitos:**
- Michi Micro Server corriendo (ej: 192.168.1.101:8500).
- Token obtenido tras pairing manual (por ahora).

**Ejecución:**

```bash
# Pairing + token refresh
python runner.py scenarios/mobile_micro.yml --server-host 192.168.1.101 --server-port 8500 --certified-by dev@example.com

# Download + sync
python runner.py scenarios/mobile_micro.yml --server-host 192.168.1.101 --server-port 8500 --token <TOKEN>
```

### Escenario C: Player → Micro Server (Import)

**Requisitos:**
- Player y Micro Server en la misma red.
- Ambos emparejados previamente.
- Player tiene tracks para importar.

**Ejecución:**

```bash
python runner.py scenarios/player_micro_import.yml --player-host 192.168.1.100 --player-port 8400 --micro-host 192.168.1.101 --micro-port 8500
```

### Escenario D: Micro Server → Stream

**Requisitos:**
- Micro Server corriendo.
- Stream simulator (o firmware real).
- Solo para certificación avanzada (no blocker beta).

**Ejecución:**

```bash
python runner.py scenarios/micro_stream_receiver.yml --micro-host 192.168.1.101 --micro-port 8500
```

### Ver los reportes

Los reportes se guardan en `tests/e2e_certification/reports/`:

```bash
ls tests/e2e_certification/reports/
cat tests/e2e_certification/reports/mobile_player_pairing-20260715.json
```

### Interpretar resultados

Cada check tiene status `pass`, `fail`, o `skip`. El reporte global es `pass` solo si todos los checks pasan.

```json
{
  "scenario": "E2E-01",
  "name": "mobile_player_pairing",
  "status": "pass",
  "checks": [
    { "name": "server_info", "status": "pass", "detail": "service: michi-music-player" },
    { "name": "pair_start", "status": "pass", "detail": "pairing_code received" }
  ],
  "errors": []
}
```

### Niveles de certificación

| Nivel | Significado |
|-------|-------------|
| NOT_TESTED | No ejecutado |
| UNIT_PASS | Pasa test unitario del proyecto |
| MOCK_PASS | Pasa contra servidor mock |
| MANUAL_PASS | Pasa manualmente con curl |
| E2E_PASS | Pasa con runner.py contra servidor real |
| FAIL | No pasa |
