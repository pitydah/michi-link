# Quality Leap Roadmap — Michi Link v1.0.0-alpha → v1.0.0-beta

## Fases

```
Fase 0: Alpha actual
├── Contrato alineado ✅
├── 34 tests de contrato ✅
├── Micro Server 69 unit tests ✅
├── Runner E2E creado ✅
└── Escenarios YAML definidos ✅

Fase 1: Player certificación (próximo)
├── Player pasa todos los contract tests
├── Mobile → Player LOCAL_E2E_PASS
├── Mobile → Player NETWORK_E2E_PASS
└── Reportes en tests/e2e_certification/reports/

Fase 2: Micro Server certificación (próximo)
├── Mobile → Micro LOCAL_E2E_PASS
├── Mobile → Micro NETWORK_E2E_PASS
├── Player → Micro import LOCAL_E2E_PASS
├── Micro autonomous LOCAL_E2E_PASS
└── Rollback import probado

Fase 3: Beta gate
├── Todos los escenarios críticos ≥ LOCAL_E2E_PASS
├── Mobile → Player ≥ NETWORK_E2E_PASS
├── Seguridad validada
├── Checklist 100% con evidencia
└── Se declara v1.0.0-beta

Fase 4: Estabilización beta
├── Tests E2E en CI/CD
├── NETWORK_E2E_PASS semanal
├── Bugfixes sin cambio de contrato
└── Music Stream avanza a MOCK_PASS
```

## Métricas de calidad

| Métrica | Alpha actual | Meta beta |
|---------|-------------|-----------|
| Contract tests | 34 | 34+ |
| Micro Server unit tests | 69 | 80+ |
| Player unit tests | — | 30+ |
| Mobile unit tests | — | 20+ |
| E2E escenarios certificados | 0 | 8 |
| Escenarios ≥ LOCAL_E2E_PASS | 0 | 8 |
| Escenarios ≥ NETWORK_E2E_PASS | 0 | 6 |
| Beta blockers resueltos | 0 | 10 |
| Reportes en e2e_certification/reports/ | 0 | 8 |

## Hitos

| Hito | Fecha estimada | Dependencias |
|------|---------------|--------------|
| Player pasa contract tests | TBD | michi-music-player |
| Mobile → Player NETWORK_E2E_PASS | TBD | Player + Mobile |
| Mobile → Micro NETWORK_E2E_PASS | TBD | Micro + Mobile |
| Player → Micro import LOCAL_E2E_PASS | TBD | Player + Micro |
| Rollback import probado | TBD | Micro |
| Beta gate abierta | TBD | Todos los anteriores |
| v1.0.0-beta declarada | TBD | Beta gate ✅ |

## Riesgos

| Riesgo | Impacto | Mitigación |
|--------|---------|------------|
| Mobile no implementa consumo de API a tiempo | Beta bloqueada | Probar con curl/scrcpy + curl |
| Player no expone endpoints REST | Beta bloqueada | Probar con Micro Server como sustituto parcial |
| Import upload requiere file system compartido | Complejidad alta | Documentar requisitos de red NFS/SMB |
| No hay hardware Stream real | Prototype perpetuo | Simulador en Python |
