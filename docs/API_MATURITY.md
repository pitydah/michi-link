> **HISTORICAL** — kept for reference only; NOT part of the active contract.

# API Maturity Model — Michi Link v1.0.0-alpha

> **HISTORICAL NOTE:** This document is superseded by `docs/IMPLEMENTATION_MATRIX.md` (per-endpoint evidence levels) and `docs/BETA_GATE.md` (beta conditions). Its "beta-ready certificado" claims for the contract were never backed by E2E certification reports (see `docs/BETA_BLOCKERS.md` and `tests/e2e_certification/reports/`). Kept for reference only.

## Niveles

| Nivel | Significado | Requisitos |
|-------|-------------|------------|
| **concept** | Idea documentada, sin implementación. | Documentación en `docs/`. |
| **prototype** | Implementación experimental, no lista para producción. | Schema + ejemplo + implementación básica. |
| **alpha** | Implementación funcional, contrato en evolución. | Schema validado, ejemplo validado, tests de contrato, implementación en al menos un proyecto. |
| **beta-ready documental** | Contrato completo y validado. Sin certificación E2E. | Misma que alpha + todas las implementaciones declaran los endpoints. |
| **beta-ready certificado** | Contrato validado + escenarios E2E certificados. | beta-ready documental + reportes E2E en escenarios críticos. |
| **beta** | En pruebas activas, sin cambios de contrato. | Beta blockers resueltos, checklist superado, CI/CD con tests E2E. |
| **stable** | Contrato congelado, solo bugfixes. | 3 meses en beta sin issues críticos, tests certificados. |

## Certification levels

| Código | Nivel | Entorno |
|--------|-------|---------|
| NOT_TESTED | No probado | — |
| UNIT_PASS | Unit test | Test aislado |
| MOCK_PASS | Mock de red | HTTP simulado |
| LOCAL_E2E_PASS | Localhost real | `127.0.0.1` |
| NETWORK_E2E_PASS | LAN real | `192.168.1.x` |
| DEVICE_E2E_PASS | Hardware real | Físico |
| FAIL | No pasa | Cualquiera |

---

## Maturity por Área

### 1. Server Info (`/server/info`, `/status`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready certificado** | Schema, docs, examples, tests 34/34, runner.py |
| Michi Music Player | **beta-ready documental** | Endpoints expuestos, contrato alineado. Pendiente E2E_PASS. |
| Michi Micro Server | **beta-ready documental** | 69 tests unitarios, contrato alineado. Pendiente E2E_PASS. |
| Michi Music Mobile | **alpha** | Sin prueba E2E real con Player ni Micro. Bloqueador beta. |

**Meta beta:** Player y Micro Server alineados con contrato, E2E manual verificado.

---

### 2. Auth Profiles

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready certificado** | AUTH_PROFILES.md completo |
| Michi Music Player (PLAYER_PASSWORD) | **beta-ready documental** | Endpoint expuesto, contrato alineado |
| Michi Micro Server (SERVER_CODE) | **beta-ready documental** | Tests unitarios, contrato alineado |
| Michi Music Mobile (detecta estrategia) | **alpha** | Sin implementación validada. Bloqueador beta. |
| Michi Music Stream (RECEIVER_BUTTON) | **concept** | Sin firmware validado |

**Meta beta:** Mobile detecta estrategia, Player y Micro Server responden auth correcto.

---

### 3. Pairing (`/pair/start`, `/pair/confirm`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready certificado** | Schemas, docs, examples |
| Michi Music Player | **beta-ready documental** | Endpoints expuestos |
| Michi Micro Server | **beta-ready documental** | Tests unitarios + integración |
| Michi Music Mobile | **alpha** | Consume, sin test E2E. Bloqueador beta. |
| Michi Music Stream (v1-lite) | **prototype** | Sin hardware real |

**Beta blocker:** Mobile ↔ Player y Mobile ↔ Micro deben funcionar.

---

### 4. Library (`/library/stats`, `/library/scan`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready** | |
| Michi Music Player | **beta-ready** (server) | Endpoints expuestos |
| Michi Micro Server | **beta-ready** (server) | Tests de stats y tracks |

**Meta beta:** Player puede escanear y publicar stats que Mobile pueda leer.

---

### 5. Tracks, Albums, Artists, Search

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready** | Pagination, filtros documentados |
| Michi Music Player | **beta-ready** (server) | Endpoints expuestos |
| Michi Micro Server | **beta-ready** (server) | Tests de tracks, search, pagination |
| Michi Music Mobile | **alpha** | Consume, sin test E2E |

**Meta beta:** Mobile puede browse library de Player y Micro Server.

---

### 6. Streaming (`/stream/{id}`, `/download/{id}`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready** | Range, 206, 416 documentados |
| Michi Music Player | **beta-ready** (server) | Range tests existentes |
| Michi Micro Server | **beta-ready** (server) | Range + download con permisos |
| Michi Music Mobile | **alpha** | Consume stream/download, sin test E2E |

**Beta blocker:** Mobile ↔ Player stream, Mobile ↔ Micro download.

---

### 7. Artwork (`/artwork/{id}`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready** | |
| Michi Music Player | **beta-ready** (server) | Sirve artwork |
| Michi Micro Server | **beta-ready** (server) | Sirve artwork |
| Michi Music Mobile | **alpha** | Consume, sin test E2E |

**Beta blocker:** Mobile puede ver artwork de Player y Micro Server.

---

### 8. Sync (`/sync/manifest`, `/sync/manifest/delta`, `/sync/state`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready** | Cursor oficial, legacy soportado |
| Michi Music Player | **beta-ready** (server) | Exporta manifest |
| Michi Micro Server | **beta-ready** (server) | Manifest + delta + state con tests |
| Michi Music Mobile | **alpha** | Consume sync, sin test E2E |

**Beta blocker:** Mobile ↔ Micro sync completo (manifest, delta, state).

---

### 9. Playback (`/playback/state`, `/playback/control`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready** | Command oficial, action legacy |
| Michi Music Player | **beta-ready** (server) | Acepta command + action |
| Michi Micro Server | **beta-ready** (server) | Tests de control + legacy |
| Michi Music Mobile | **alpha** | Envía control, sin test E2E |

**Beta blocker:** Mobile ↔ Player control, Mobile ↔ Micro control.

---

### 10. Queue (`/queue`, `/queue/items`, `/queue/jump`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **beta-ready** | |
| Michi Music Player | **beta-ready** (server) | Endpoints expuestos |
| Michi Micro Server | **beta-ready** (server) | Items + jump con tests |
| Michi Music Mobile | **alpha** | Consume, sin test E2E |

**Beta blocker:** Mobile ↔ Player queue, Mobile ↔ Micro queue.

---

### 11. Import (`/import/session`, `/import/upload`, `/import/commit`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **alpha** | Schemas existen, sin examples de upload real |
| Michi Music Player | **prototype** | Sin test E2E de import |
| Michi Micro Server | **alpha** | Session + upload + commit con tests |

**Beta blocker:** Player → Micro Server import completo.

---

### 12. Receivers (`/receivers/*`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **alpha** | Schemas, docs |
| Michi Music Player | **planned** | Sin implementación |
| Michi Micro Server | **partial** | CRUD básico sin integración real |
| Michi Music Stream | **prototype** | Sin hardware real |

**Meta beta:** No es blocker, pero debe progresar a alpha coordinado.

---

### 13. Events (`/events` WebSocket)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **alpha** | Documentado como secundario |
| Michi Music Player | **stub** | Endpoint declarado, sin eventos |
| Michi Micro Server | **alpha** | Broadcast básico de playback.state_changed |
| Michi Music Mobile | **planned** | Sin consumo de eventos |

**Meta beta:** No es blocker. Prioridad baja.

---

### 14. Rooms (`/rooms/*`)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **alpha** | Schemas, docs |
| Todos | **planned** | Sin implementación real |

**Meta beta:** No es blocker. Pospuesto para v1.1.

---

### 15. v1-lite (Receivers físicos)

| Proyecto | Nivel | Notas |
|----------|-------|-------|
| Michi Link (contrato) | **alpha** | RECEIVERS_V1_LITE.md completo |
| Michi Micro Server | **partial** | Consume, endpoints declarados |
| Michi Music Stream | **prototype** | Sin firmware validado en hardware |

**Beta blocker:** Micro Server ↔ Stream para poder avanzar a prototype real.

---

## Resumen Visual

```
Área                    Concept  Proto  Alpha  Beta-ready  Beta  Stable
─────────────────────────────────────────────────────────────────────
Server Info                               ●        ●
Auth Profiles                             ●        ●
Pairing                                   ●        ●
Library                                   ●        ●
Tracks/Albums/Search                      ●        ●
Streaming                                 ●        ●
Artwork                                   ●        ●
Sync                                      ●        ●
Playback                                  ●        ●
Queue                                     ●        ●
Import                       ●
Receivers            ●        ●
Events                                     ●
Rooms                          ●
v1-lite                    ●    ●
```

- **●** = nivel alcanzado por al menos un proyecto
- Sin marca = no ha alcanzado ese nivel aún

### Leyenda de proyectos

| Marca | Proyecto |
|-------|----------|
| **beta-ready documental** | Player y Micro Server tienen implementación completa del contrato |
| **beta-ready certificado** | Player y Micro Server pasaron E2E_PASS con runner.py |
| **alpha** | Mobile consume pero no hay E2E certificado |
| **prototype** | Music Stream sin hardware real |

---

## Próximos Pasos para Beta

1. Resolver beta blockers (B-01 a B-08 en BETA_BLOCKERS.md).
2. Pasar checklist de beta readiness con evidencia E2E.
3. Automatizar E2E tests para escenarios A, B, C.
4. Certificar al menos un flujo completo por escenario.
5. Congelar contrato v1.0.0-beta.
