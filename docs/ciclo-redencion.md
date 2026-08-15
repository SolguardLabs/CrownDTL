# Ciclo de redención

## Admisión

Una orden declara cuenta, bóveda, importe en shares, carril y ventana. El motor comprueba existencia, importe mínimo y máximo, saldo, elegibilidad del nivel, profundidad de cola, límite diario y, para prioridad, capacidad disponible. La cotización queda fijada en el ticket para que el resultado no dependa de lecturas posteriores.

```mermaid
flowchart TB
    R["Solicitud"] --> N["Normalizar campos"]
    N --> B{"Saldo suficiente"}
    B -->|no| X["Rechazar"]
    B -->|sí| L{"Límite diario"}
    L -->|no| X
    L -->|sí| P{"Prioridad"}
    P -->|sí| C{"Elegibilidad y capacidad"}
    C -->|no| X
    C -->|sí| T["Crear ticket"]
    P -->|no| T
    T --> Q["Encolar"]
```

El recibo contiene `redemption_id`, activos cotizados, día de desbloqueo, ventana y carril. El identificador es la referencia estable para operaciones posteriores; el adaptador debe asociar su clave de idempotencia con ese recibo.

## Orden y procesado

```mermaid
sequenceDiagram
    participant O as Operador
    participant E as Motor
    participant Q as Cola
    participant K as Libro de derechos
    O->>E: Procesar bóveda con máximo N
    E->>Q: Obtener orden estable
    loop hasta N
      Q-->>E: Siguiente ticket
      E->>K: Registrar derecho y madurez
      E->>E: Marcar pendiente de desbloqueo
    end
    E-->>O: Identificadores procesados
```

El carril prioritario precede al estándar. Dentro de prioridad, los niveles institucional y VIP conservan su orden económico; la secuencia resuelve empates. `max_tickets` limita el trabajo por ejecución y permite dimensionar ciclos operativos predecibles.

## Estados

```mermaid
stateDiagram-v2
    state "Queued" as Q
    state "PendingUnlock" as P
    state "Cancelled" as C
    state "Withdrawn" as W
    [*] --> Q
    Q --> P: process
    Q --> C: cancel
    P --> W: withdraw tras madurez
    C --> [*]
    W --> [*]
```

Las transiciones terminales no se reabren. Una cancelación solo es admisible mientras el ticket sea cancelable; una retirada solo se admite tras la madurez declarada. El journal conserva los eventos que explican cada cambio.

## Conciliación por cierre

Al terminar un ciclo se captura `ProtocolReport`. Para cada bóveda se contrastan reserva, shares, activos pendientes, profundidad de cola, derechos abiertos y recuento de tickets por estado. Para cada cuenta se contrastan posiciones de shares y activos.

Lista mínima de cierre:

1. La cola procesada coincide con el máximo solicitado y el orden esperado.
2. Los derechos abiertos coinciden con posiciones pendientes de desbloqueo.
3. La reserva líquida cubre la reserva requerida del modelo de capital.
4. Los contadores de límite y capacidad explican el flujo del día.
5. El número de eventos avanzó conforme a las transiciones confirmadas.

Los escenarios `order`, `limits`, `cancel` y `withdrawals` ofrecen salidas JSON estables para automatizar estas comprobaciones sin depender de servicios externos.
