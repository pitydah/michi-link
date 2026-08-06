# Contract Governance — Michi Link API v1.0.0-alpha

## Principios

1. **michi-link es la única fuente de verdad.** Ningún repositorio de aplicación puede modificar el contrato unilateralmente.
2. **Todo cambio de payload actualiza 4 artefactos:** schema, docs, examples, tests.
3. **Cambios incompatibles se guardan para v1.1/v2.** Durante v1.0.x solo bugfixes y adiciones compatibles.
4. **La matriz de implementación refleja la realidad.** Si un proyecto no implementa un endpoint, la matriz lo muestra con su nivel de evidencia real (`NOT_TESTED`/`UNIT_PASS`/`MOCK_PASS`/`LOCAL_E2E_PASS`/`NETWORK_E2E_PASS`/`DEVICE_E2E_PASS`/`FAIL`), nunca como `stable`/`DONE`/`manual`.

## Órganos de Decisión

### Cambios compatibles (v1.0.x)
- Decide: maintainer de michi-link, con revisión de al menos un maintainer de otro repo.
- Proceso: PR en michi-link → revisión → merge → notificación a otros repos.

### Cambios incompatibles (v1.1 / v2)
- Decide: todos los maintainers del ecosistema (mínimo 3 de 4 repos).
- Proceso: RFC en michi-link → discusión 2 semanas → votación → implementación en michi-link → migración en repos.

### Bugfixes urgentes
- Decide: maintainer de michi-link, notificación posterior.
- Proceso: PR con fix → merge rápido → notificación a repos afectados.

## Actualización de la Matriz

Cuando un proyecto implementa un nuevo endpoint o cambia su estado:

1. El maintainer del proyecto abre un PR en michi-link.
2. Actualiza la fila correspondiente en IMPLEMENTATION_MATRIX.md.
3. Si aplica, actualiza BETA_READINESS_CHECKLIST.md.
4. El PR se revisa y mergea.

## Versionado Semántico

| Componente | Valor | Contrato |
|------------|-------|----------|
| Contract version | `api_version: "v1"` | Permanente para compatibilidad |
| Lite subset | `api_version: "v1-lite"` | Solo para receivers (subconjunto de v1) |
| App version | `version` | Versión de la aplicación; NO es la versión del contrato |
| ~~`michi_link_version`~~ | Retirado en v1 | No es parte del contrato; rechazado por `additionalProperties: false` | <!-- michi-policy:exclude -->

- `api_version: "v1"` es la versión del contrato y es permanente para compatibilidad: mientras un dispositivo declare `"v1"`, debe ser compatible con cualquier otro `"v1"`. La versión del contrato NO cambia con versiones de app.
- `michi_link_version` fue retirado en v1. No es un campo existente ni planificado: los servidores rechazan payloads que lo incluyan con `INVALID_REQUEST` (ver Deprecation más abajo). <!-- michi-policy:exclude -->

## Single Source of Truth

Michi Link is the single source of truth for the ecosystem contract. No application repository may modify the contract unilaterally: every payload change updates the 4 artifacts together (schema, docs, examples, tests).

## Consumers Must Not Invent Fields

Consumers and producers MUST NOT add fields beyond the canonical schemas. Active schemas enforce `additionalProperties: false`; a payload carrying unknown fields (e.g. the retired `michi_link_version`) is rejected with `INVALID_REQUEST`. A feature that needs a new field must go through the change process above, never through a local extension. <!-- michi-policy:exclude -->

## Breaking Changes

Breaking changes (removing a field, changing a type, making a field required, removing an endpoint, changing an error code) are accumulated for v1.1/v2 and require the full decision process (RFC in michi-link → 2-week discussion → vote among maintainers → implementation → migration in consuming repos). During v1.0.x only compatible additions and bugfixes are allowed.

## Backward Compatibility

- `api_version: "v1"` is permanent: any two "v1" devices must interoperate regardless of app versions.
- New fields are always optional; new enum values must not break parsing.
- Full rules: `docs/CONTRACT_BACKWARD_COMPATIBILITY.md`.

## Deprecation

- A **deprecated** field stays accepted during a documented grace period.
- A **retired** field is removed from the contract: requests carrying it are rejected explicitly with `INVALID_REQUEST` — never silently ignored, and never emitted by servers.
- `michi_link_version` is retired as of v1. `server-info` validation rejects it (negative test in `tests/contract/validate.js`). <!-- michi-policy:exclude -->

## Evidence Requirements

Implementation claims MUST use the evidence levels `NOT_TESTED` / `UNIT_PASS` / `MOCK_PASS` / `LOCAL_E2E_PASS` / `NETWORK_E2E_PASS` / `DEVICE_E2E_PASS` / `FAIL`, tracked per endpoint in `docs/IMPLEMENTATION_MATRIX.md` and per item in `docs/BETA_READINESS_CHECKLIST.md`. Levels like `stable`, `DONE` or `manual` are NOT evidence levels. Unmeasured claims stay `NOT_TESTED`.

## Beta vs Stable

- **Beta**: all beta-gate conditions met (see `docs/BETA_GATE.md`): Mobile↔Player and Mobile↔Micro at `NETWORK_E2E_PASS` or `DEVICE_E2E_PASS`, Player→Micro import at least `LOCAL_E2E_PASS`, Micro autonomous playback at least `LOCAL_E2E_PASS`.
- **Stable**: contract frozen, only bugfixes; declared only after a beta period backed by certified E2E evidence.

## Resolución de Disputas

1. Discutir en el PR o issue de michi-link.
2. Si no hay consenso en 1 semana, el maintainer de michi-link decide.
3. Las decisiones se documentan en el issue.

## Herramientas de Verificación

- `npm test` en `tests/contract/` — valida schemas contra examples (109 checks).
- `tests/e2e_certification/reports/` — reportes de certificación E2E.
- `python3 scripts/contract-policy.py` — escanea tokens retirados; los docs con marcador `HISTORICAL` en línea 1 quedan excluidos.
- `docs/BETA_READINESS_CHECKLIST.md` — checklist de beta.
