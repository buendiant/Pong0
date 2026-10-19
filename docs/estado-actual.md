# Estado del proyecto Pong0

Revisión: 2026-10-19 (America/Bogota), después de implementar las fases 4 y 5.

## Repositorio local

- Repositorio: https://github.com/buendiant/Pong0
- Carpeta: `C:\Users\Asus\Documents\ChatGPT\Pong0`.
- Rama: `master`, siguiendo `origin/master`.
- Base descargada: `d84d32e7bcc65abce58cb157c85548a3d825c41d` (2026-10-17).
- La estructura original se conserva. La implementación actual es local.
- `MEMORY.md` conserva el contenido previo a esta implementación, por solicitud del usuario; su versión Alpha 0.4 es un registro histórico y no refleja las fases nuevas.

## Estado contrastado con el código

| Fase | Estado | Evidencia |
| --- | --- | --- |
| 1. Demo local | Implementada | Cliente Macroquad y NetworkBridge; compilación comprobada, sin verificación visual/FPS en esta sesión. |
| 2. Especificación binaria | Documentada y precisada | Cabecera, siete mensajes y reglas de registro, pérdida, secuencias y expiración en `docs/protocolo.md`. |
| 3. UDP y logs | Implementada | RX/TX y eventos registrados en consola y archivo. |
| 4. Serialización manual | Completada | `src/protocol.rs`; siete pruebas con bytes exactos, ida/vuelta y rechazo de paquetes inválidos. |
| 5. Autoridad y concurrencia | Completada | `src/game.rs`, `src/server.rs`; cinco pruebas del motor y ocho con sockets UDP reales, incluidas dos parejas simultáneas. |
| 6. Integración con GUI | Fase actual, pendiente | El cliente todavía usa `spawn_offline_demo_worker`. Falta trabajador UDP, mapeo de estados, RTT y dos clientes gráficos por red. |
| 7. AWS y entrega | Pendiente | Despliegue EC2, pruebas por Internet, README final y Makefile. |

Las fases 4 y 5 están validadas con clientes UDP automatizados en localhost.
El juego gráfico aún no constituye un MVP online; no se marca BETA.

## Verificación local

La incompatibilidad inicial con Rust 1.82.0 quedó resuelta instalando una toolchain compatible. El repositorio fija **Rust 1.90.0** mediante `rust-toolchain.toml`, manteniendo el compilador global predeterminado.

Comprobaciones realizadas:

- 20 pruebas aprobadas: 7 de protocolo, 5 del motor y 8 de integración UDP.
- Compilación de comprobación del servidor sin features gráficas y del cliente.
- Clippy del servidor, biblioteca headless y pruebas con advertencias tratadas como errores.
- Uso de directorios de salida separados dentro de `target/` para conservar los artefactos originales.

La ejecución visual de la GUI, el rendimiento bajo carga y la conectividad entre equipos/AWS todavía no están verificados. Las pruebas UDP necesitaron ejecución fuera del aislamiento de red del entorno.

## Continuación

La explicación paso a paso y los comandos están en `docs/fases-4-5.md`.
La fase 6 debe integrar el cliente mediante NetworkBridge, repetir entradas frescas cada 100 ms como máximo y consumir los estados autoritativos. Se debe decidir cómo representar `Ready`, `Quit`, RTT y campos visuales que no viajan en STATE_SYNC.

La limpieza de los 106 archivos de `target/` ya versionados sigue pendiente.
El enunciado permite C/Rust para el servidor, pero contiene una nota posterior que menciona solo C; esa contradicción debe aclararse con el docente.

La entrega indicada en `ProyectoPongN1.md` es el **22 de octubre de 2026 a las 23:59**, sin zona horaria explícita. El documento prohíbe commits posteriores a ese límite.
