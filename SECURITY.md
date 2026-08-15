# Política de seguridad

## Versiones mantenidas

| Versión | Estado | Canal |
| --- | --- | --- |
| `1.0.x` | Mantenida | `production` |
| `< 1.0.0` | Fuera de soporte | — |

El tag de una publicación debe apuntar al mismo commit que `main` y `production`. El workflow `Release integrity` verifica esa cadena en cada promoción.

## Comunicación responsable

Utiliza **Security → Report a security issue** en GitHub para comunicar de forma privada cualquier comportamiento que afecte a autorización, integridad contable, disponibilidad o confidencialidad. No abras una issue pública con detalles operativos.

Incluye versión y commit, precondiciones, componente, impacto, evidencia mínima y una propuesta de propiedad que debería preservarse. No incluyas credenciales, claves, datos personales ni material de terceros. El equipo confirmará recepción, clasificará el caso y coordinará la corrección y la divulgación.

## Fronteras de confianza

```mermaid
flowchart TB
    subgraph U["Zona de integración"]
      C["Cliente institucional"]
    end
    subgraph A["Zona de aplicación"]
      API["Adaptador HTTPS"]
      E["CrownEngine"]
    end
    subgraph S["Zona de estado"]
      V["Bóvedas"]
      P["Políticas"]
      J["Journal"]
    end
    C -->|"orden idempotente"| API
    API -->|"entrada normalizada"| E
    E --> V
    E --> P
    E --> J
```

```mermaid
flowchart LR
    I["Cambio administrativo"] --> H["Hash canónico"]
    H --> Q["Aprobaciones de quorum"]
    Q --> T["Timelock"]
    T --> X{"Predecesor ejecutado"}
    X -->|sí| E["Ejecución"]
    X -->|no| R["Rechazo cerrado"]
```

## Controles obligatorios

- Entradas normalizadas, dominios explícitos e identificadores acotados.
- Operaciones aritméticas comprobadas y redondeo documentado.
- HTTPS, rechazo de redirecciones e idempotencia para escrituras del cliente.
- Quorum, timelock, caducidad y relación de predecesores para administración.
- Conciliación entre reservas, shares, tickets, derechos y journal.
- Dependencias bloqueadas y auditoría automática en cada cambio.
- Revisión mediante CODEOWNERS para núcleo, CI y política de seguridad.

## Matriz de revisión

| Área | Propiedad | Evidencia automática |
| --- | --- | --- |
| Reservas | Cobertura suficiente bajo estrés | pruebas de `capital` |
| Redenciones | Transiciones válidas y orden estable | pruebas Rust de integración |
| Cliente | Precisión entera e idempotencia | pruebas Node del SDK |
| Gobierno | Operaciones únicas, maduras y aprobadas | pruebas de `governance` |
| Promoción | Referencias en un único commit | workflow de integridad |

## Respuesta operativa

```mermaid
sequenceDiagram
    participant R as Remitente
    participant S as Equipo de seguridad
    participant E as Ingeniería
    participant O as Operaciones
    R->>S: Comunicación privada
    S->>S: Triage y severidad
    S->>E: Reproducción acotada
    E->>S: Cambio y pruebas
    S->>O: Autorización de promoción
    O->>O: Verificación de referencias
    S-->>R: Resolución coordinada
```
