# Security Policy

## Modelo de seguridad

Crown DTL asume que las cuentas registradas mantienen portfolios de shares y
assets dentro del motor local. El vault conserva reservas del activo subyacente,
emite shares y procesa redenciones mediante tickets con estado explicito.

Las rutas sensibles son:

- solicitud de redencion;
- consumo de limite diario;
- admision en cola prioritaria;
- cancelacion de tickets;
- procesado de cola;
- unlock y withdrawal;
- reconciliacion de shares y claims abiertos.

## Invariantes esperadas

- Las shares totales del vault coinciden con las shares agregadas de cuentas.
- Las redenciones usan importes enteros con aritmetica comprobada.
- Las colas priorizadas respetan lane, tier y orden de llegada.
- Los limites diarios se consumen y liberan de forma determinista.
- La capacidad prioritaria se controla por vault y day.
- Los withdrawals solo se completan cuando la ventana de unlock esta madura.
- La reserva del vault debe cubrir los claims abiertos.

## Validaciones automatizadas

La suite local ejecuta:

```bash
cargo fmt --all -- --check
cargo build --all-targets --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
node --test tests/node/*.test.js
```

Los tests JavaScript cubren orden de cola, limites diarios, cancelaciones y
withdrawals. Los tests Rust ejercitan flujos de API publica y reconciliacion de
estado.

## Dependencias

El crate no usa dependencias Rust externas. Los tests JavaScript usan modulos
nativos de Node.js. Dependabot esta configurado para Cargo, npm y GitHub
Actions para mantener el repositorio alineado con cambios de tooling.

## Alcance de revision

La revision debe centrarse en:

- consistencia de accounting entre accounts, claims y vaults;
- orden temporal entre request, cancel, process y withdraw;
- integridad de limites por usuario;
- capacidad disponible por day;
- estados de ticket y claims durante unlock windows;
- reportes e invariantes emitidos por el ledger.

## Reporte interno

Un reporte debe incluir:

- resumen ejecutivo;
- impacto economico;
- precondiciones;
- ubicacion exacta;
- secuencia de reproduccion;
- mitigacion propuesta;
- tests recomendados.
