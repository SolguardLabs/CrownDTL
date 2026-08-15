# Arquitectura de CrownDTL

## Objetivo

CrownDTL separa decisión, contabilidad y observación. `CrownEngine` es la frontera transaccional: recibe una orden ya autenticada, consulta políticas, modifica los registros económicos y ejecuta las invariantes antes de devolver un recibo. Los módulos no realizan llamadas de red ni leen el reloj del sistema; el día de epoch se inyecta para que cada transición sea reproducible.

## Capas

```mermaid
flowchart TB
    SDK["SDK JavaScript"] --> ADP["Adaptador de servicio"]
    ADP --> ENG["CrownEngine"]
    ENG --> DOM["Dominio: cuentas, bóvedas y tickets"]
    ENG --> CTRL["Control: políticas, colas y ventanas"]
    ENG --> ACC["Contabilidad: ledger y derechos"]
    ACC --> REP["Reportes deterministas"]
```

El adaptador no forma parte del núcleo y debe convertir autenticación y transporte a tipos del dominio. El motor conserva el orden de escritura. `Amount`, `Rate` y `BasisPoints` centralizan límites y operaciones comprobadas. Los identificadores tipados evitan mezclar cuentas, activos, bóvedas, ventanas y redenciones.

## Dependencias internas

```mermaid
flowchart LR
    IDs["ids"] --> ACCOUNTS["accounts"]
    IDs --> VAULT["vault"]
    AMOUNT["amount"] --> ACCOUNTS
    AMOUNT --> POLICY["policy"]
    CLOCK["clock"] --> POLICY
    POLICY --> ENGINE["engine"]
    VAULT --> ENGINE
    PRIORITY["priority"] --> ENGINE
    QUEUE["queue"] --> ENGINE
    ENGINE --> REPORTS["reports"]
    CAPITAL["capital"] --> REPORTS
    GOVERNANCE["governance"] --> ENGINE
```

`PriorityBook` representa derechos pendientes; `VaultState` mantiene reservas, shares y tickets; `ProtocolLedger` conserva límites, capacidad y eventos. La orquestación debe revisar los tres planos como una sola unidad económica.

## Escritura transaccional

```mermaid
sequenceDiagram
    participant A as Adaptador
    participant E as CrownEngine
    participant P as Políticas
    participant V as Bóveda
    participant L as Ledger
    A->>E: Orden normalizada
    E->>P: Validar límite y capacidad
    P-->>E: Admisión
    E->>V: Aplicar transición
    E->>L: Registrar movimiento y evento
    E->>E: Comprobar invariantes
    E-->>A: Recibo determinista
```

Si una operación falla, el llamador no debe publicar un recibo parcial. Para persistencia externa, se recomienda ejecutar sobre una copia de estado, confirmar invariantes y escribir mediante compare-and-swap con el número de versión observado.

## Escalabilidad y aislamiento

La clave natural de partición es `VaultId`. Las cuentas pueden mantener posiciones en varias bóvedas, por lo que los procesos de conciliación agregada deben trabajar sobre snapshots consistentes. Las colas usan secuencias monotónicas por motor; un adaptador distribuido debe serializar escrituras de una misma bóveda o emplear control optimista.

La lectura de reportes no concede autoridad de escritura. La configuración de política se administra por operaciones de gobierno con identidad canónica y no mediante parámetros libres en cada solicitud.
