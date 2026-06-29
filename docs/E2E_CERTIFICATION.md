# E2E Certification — Michi Link API v1.0.0-alpha

## Propósito

Certificar que cada escenario end-to-end funciona entre dos proyectos Michi antes de declarar beta.

## Escenarios Oficiales

| ID | Escenario | Server | Client | Prioridad |
|----|-----------|--------|--------|-----------|
| E2E-01 | Mobile ↔ Player pairing | Player | Mobile | Crítica |
| E2E-02 | Mobile ↔ Player stream | Player | Mobile | Crítica |
| E2E-03 | Mobile ↔ Player playback control | Player | Mobile | Crítica |
| E2E-04 | Mobile ↔ Micro Server pairing | Micro Server | Mobile | Crítica |
| E2E-05 | Mobile ↔ Micro Server download | Micro Server | Mobile | Crítica |
| E2E-06 | Mobile ↔ Micro Server playback control | Micro Server | Mobile | Crítica |
| E2E-07 | Player → Micro Server import | Player | Micro Server | Alta |
| E2E-08 | Micro Server autonomous playback | Micro Server | — | Alta |
| E2E-09 | Micro ↔ Stream receiver simulator | Micro Server | Simulator | Media |

## Formato de Reporte

Cada certificación produce un archivo JSON en `tests/e2e_contract/reports/`:

```json
{
  "scenario": "E2E-01",
  "name": "mobile_player_pairing",
  "status": "pass",
  "server": { "type": "michi-music-player", "version": "0.1.0" },
  "client": { "type": "michi-mobile", "version": "0.1.0" },
  "timestamp": "2026-07-15T14:00:00Z",
  "checks": [
    { "name": "discovery", "status": "pass", "detail": "UDP announce recibido" },
    { "name": "server_info", "status": "pass", "detail": "service: michi-music-player" },
    { "name": "pair_start", "status": "pass", "detail": "pairing_code recibido" },
    { "name": "pair_confirm", "status": "pass", "detail": "token obtenido" }
  ],
  "errors": [],
  "certified_by": "desarrollador@example.com",
  "certified_at": "2026-07-15T14:05:00Z"
}
```

## Proceso de Certificación

1. Ejecutar el escenario manualmente o con script.
2. Generar reporte JSON según schema.
3. Agregar reporte a `tests/e2e_contract/reports/{scenario}-{fecha}.json`.
4. Actualizar `BETA_READINESS_CHECKLIST.md` con enlace al reporte.
5. Si falla: crear issue en el repositorio responsable con el reporte adjunto.

## Requisitos Mínimos para Certificar

- Escenario ejecutado en red local real (no localhost).
- Reporte generado sin errores.
- Al menos dos ejecuciones exitosas en días diferentes.
- Captura de pantalla o video opcional para escenarios con UI.
