# Compatibilidad de Versiones

## Formato de Versión

```
major.minor.patch
```

| Componente | Cambio                                              | Ejemplo     |
|------------|------------------------------------------------------|-------------|
| **Major**  | Cambios incompatibles con versiones anteriores       | 2.0.0       |
| **Minor**  | Adiciones compatibles hacia atrás                    | 1.3.0       |
| **Patch**  | Correcciones de errores compatibles                  | 1.2.5       |

## Reglas

### Breaking Changes (Major)

- Eliminación o renombrado de endpoints.
- Cambio en el formato de solicitud o respuesta.
- Eliminación de campos obligatorios.
- Cambio en códigos de error.

### Non-Breaking Additions (Minor)

- Nuevos endpoints.
- Nuevos campos opcionales en respuestas.
- Nuevos tipos de evento.
- Nuevos parámetros opcionales en solicitudes.

### Patches

- Corrección de errores.
- Mejoras de rendimiento.
- Seguridad.

## Garantía de Retrocompatibilidad

Dentro de la misma versión **major**, todas las versiones **minor** y **patch** son retrocompatibles. Un cliente diseñado para v1.x.x funcionará con cualquier servidor v1.y.z.

## v1-lite y v1

**v1-lite NO es una versión diferente.** Es un subconjunto del protocolo v1. Todo receiver v1-lite puede comunicarse con un servidor v1. Los endpoints de v1-lite son un subconjunto de los endpoints de v1.

```
v1 ──────────────────────────────
     ├── Endpoints completos
     └── v1-lite (subconjunto)
          ├── /receivers/*
          ├── /heartbeat
          └── /session/*
```

## Detección de Características

Los clientes pueden descubrir las capacidades del servidor consultando `/server/info`.

`GET /api/v1/server/info`

```json
{
  "version": "1.5.2",
  "name": "michi-link-server",
  "capabilities": {
    "playback": true,
    "library": true,
    "queue": true,
    "multiroom": true,
    "events": true,
    "receivers_v1_lite": true,
    "equalizer": true,
    "crossfade": true,
    "high_res_audio": false
  },
  "features": {
    "max_bit_depth": 16,
    "max_sample_rate": 48000,
    "max_receivers": 10,
    "max_rooms": 20
  }
}
```

Los clientes deben usar `capabilities` para adaptar su interfaz según lo que el servidor soporte.

## Política de Deprecación

Los endpoints marcados como **deprecated** se mantienen funcionales durante **2 versiones minor** antes de ser eliminados.

| Versión | Estado del Endpoint    |
|---------|------------------------|
| 1.5.0   | Deprecado (aviso en docs) |
| 1.6.0   | Deprecado (warning en respuesta) |
| 1.7.0   | Eliminado              |

El servidor incluye el header `Warning: 299 - "endpoint deprecated"` en respuestas de endpoints deprecados.

## Tabla de Versiones Mínimas

| Componente             | Versión Mínima | Compatible con |
|------------------------|----------------|----------------|
| Cliente Web            | 1.0.0          | Servidor ≥ 1.0.0 |
| Cliente Móvil Android  | 1.2.0          | Servidor ≥ 1.0.0 |
| Cliente Móvil iOS      | 1.3.0          | Servidor ≥ 1.0.0 |
| Receiver Standard      | 1.0.0          | Servidor ≥ 1.0.0 |
| Receiver Hi-Fi         | 2.0.0          | Servidor ≥ 1.5.0 |
| SDK Cliente            | 1.0.0          | Servidor ≥ 1.0.0 |
| CLI                    | 1.1.0          | Servidor ≥ 1.0.0 |

### Notas

- Los receivers Standard con firmware ≥ 1.0.0 funcionan con cualquier servidor v1.x.x.
- Los receivers Hi-Fi requieren servidor ≥ 1.5.0 por soporte de alta resolución.
- Las apps móviles deben actualizarse si el servidor incrementa major, pero dentro de la misma major no requieren actualización.
