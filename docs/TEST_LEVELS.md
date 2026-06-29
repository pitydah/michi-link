# Test Levels — Michi Link API v1.0.0-alpha

## Jerarquía de certificación

Cada ítem del checklist puede estar en uno de estos niveles. El nivel requerido para beta depende del escenario (ver BETA_GATE.md).

```
                         ┌──────────────┐
                         │   FAIL       │
                         │  (no pasa)   │
                         └──────┬───────┘
                                │
                    ┌───────────┴───────────┐
                    │                       │
              ┌─────┴─────┐         ┌───────┴────────┐
              │ NOT_TESTED │         │   UNIT_PASS    │
              │ (no ejec.) │         │ (test aislado) │
              └───────────┘         └───────┬────────┘
                                           │
                                    ┌──────┴──────┐
                                    │  MOCK_PASS  │
                                    │  (mock net) │
                                    └──────┬──────┘
                                           │
                              ┌────────────┴────────────┐
                              │                         │
                     ┌────────┴────────┐       ┌────────┴────────┐
                     │  LOCAL_E2E_PASS  │       │ NETWORK_E2E_PASS│
                     │ (localhost test) │       │  (LAN test)     │
                     └─────────────────┘       └─────────────────┘
                              │                         │
                              └──────────┬──────────────┘
                                         │
                                ┌────────┴────────┐
                                │ DEVICE_E2E_PASS  │
                                │ (hardware real)  │
                                └─────────────────┘
```

## Definiciones

| Nivel | Código | Entorno | Evidencia mínima |
|-------|--------|---------|------------------|
| No probado | NOT_TESTED | — | — |
| Pasa test unitario aislado | UNIT_PASS | `cargo test` / `pytest` | Log de test CI |
| Pasa contra mock de red | MOCK_PASS | Mock HTTP local | Test con server simulado |
| Pasa localhost real | LOCAL_E2E_PASS | `localhost:PUERTO` | Reporte runner.py |
| Pasa en red local | NETWORK_E2E_PASS | `192.168.1.x:PUERTO` | Reporte runner.py |
| Pasa en hardware real | DEVICE_E2E_PASS | Red física + dispositivos | Reporte runner.py + video opcional |
| No pasa | FAIL | Cualquiera | Log de error |

## ¿Qué nivel necesita cada escenario para beta?

| Escenario | Mínimo para beta | Ideal |
|-----------|-----------------|-------|
| Mobile ↔ Player pairing | NETWORK_E2E_PASS | DEVICE_E2E_PASS |
| Mobile ↔ Player stream | NETWORK_E2E_PASS | DEVICE_E2E_PASS |
| Mobile ↔ Player playback | NETWORK_E2E_PASS | DEVICE_E2E_PASS |
| Mobile ↔ Micro pairing | NETWORK_E2E_PASS | DEVICE_E2E_PASS |
| Mobile ↔ Micro download | NETWORK_E2E_PASS | DEVICE_E2E_PASS |
| Mobile ↔ Micro playback | NETWORK_E2E_PASS | DEVICE_E2E_PASS |
| Player → Micro import | LOCAL_E2E_PASS | NETWORK_E2E_PASS |
| Micro autonomous | LOCAL_E2E_PASS | NETWORK_E2E_PASS |
| Micro ↔ Stream | MOCK_PASS | DEVICE_E2E_PASS |
