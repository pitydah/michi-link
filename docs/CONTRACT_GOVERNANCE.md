# Contract Governance — Michi Link API v1.0.0-alpha

## Principios

1. **michi-link es la única fuente de verdad.** Ningún repositorio de aplicación puede modificar el contrato unilateralmente.
2. **Todo cambio de payload actualiza 4 artefactos:** schema, docs, examples, tests.
3. **Cambios incompatibles se guardan para v1.1/v2.** Durante v1.0.x solo bugfixes y adiciones compatibles.
4. **La matriz de implementación refleja la realidad.** Si un proyecto no implementa un endpoint, la matriz lo muestra como planned/stub/not-applicable.

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

| Componente | Versión | Contrato |
|------------|---------|----------|
| Michi Link protocol | 1.0.0-alpha | `michi_link_version` en server/info |
| API version | v1 | `api_version` en server/info |
| App version | 0.1.0 | `version` en server/info |

- `api_version: "v1"` significa compatibilidad con v1.x del protocolo.
- `michi_link_version: "1.0.0-alpha"` es la versión exacta del contrato.

## Resolución de Disputas

1. Discutir en el PR o issue de michi-link.
2. Si no hay consenso en 1 semana, el maintainer de michi-link decide.
3. Las decisiones se documentan en el issue.

## Herramientas de Verificación

- `npm test` en `tests/contract/` — valida schemas contra examples.
- `tests/e2e_contract/reports/` — reportes de certificación E2E.
- `docs/BETA_READINESS_CHECKLIST.md` — checklist de beta.
