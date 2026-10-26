# Estado del proyecto Pong0

Revisión: 2026-10-25 (America/Bogota), después de implementar el cliente de fase 6.

## Repositorio local

- Repositorio: https://github.com/buendiant/Pong0
- Carpeta: `C:\Users\Asus\Documents\ChatGPT\Pong0`.
- Rama: `master`, siguiendo `origin/master`.
- Base sincronizada: `b445013` (Alpha 0.5). El fetch y pull del 2026-10-25 confirmaron que no había novedades remotas.
- Fase 6 se registra en un commit local de Alpha 0.6, solicitado por el usuario después de verificar los cambios; push pendiente.
- `MEMORY.md` registra Alpha 0.6 y el proceso de implementación y verificación de la fase 6.

## Estado contrastado con el código

| Fase | Estado | Evidencia |
| --- | --- | --- |
| 1. Demo local | Implementada | Cliente Macroquad y NetworkBridge; demo explícita con `--demo`. |
| 2. Especificación binaria | Documentada y precisada | Cabecera y diez mensajes: siete iniciales más PING/PONG/LEAVE, con reglas de procedimiento. |
| 3. UDP y logs | Implementada | RX/TX y eventos registrados en consola y archivo. |
| 4. Serialización manual | Completada | `src/protocol.rs`; siete pruebas con bytes exactos, ida/vuelta y rechazo de paquetes inválidos. |
| 5. Autoridad y concurrencia | Completada | `src/game.rs`, `src/server.rs`; cinco pruebas del motor y ocho con sockets UDP reales, incluidas dos parejas simultáneas. |
| 6. Integración con GUI | Implementada y verificada localmente | Worker UDP real, formulario/CLI, estados, RTT, salida y reconexión; ocho pruebas específicas y partida completa entre dos ventanas. Aceptación entre dos equipos físicos pendiente. |
| 7. AWS y entrega | Pendiente | Despliegue EC2, pruebas por Internet, README final y Makefile. |

El juego gráfico ya se conecta al servidor y permite disputar una partida completa.
Se conserva Alpha 0.6 hasta acreditar la aceptación entre equipos físicos; no se
declara superada esa comprobación con pruebas en un único computador.

## Verificación local

La incompatibilidad inicial con Rust 1.82.0 quedó resuelta instalando una toolchain compatible. El repositorio fija **Rust 1.90.0** mediante `rust-toolchain.toml`, manteniendo el compilador global predeterminado.

Comprobaciones realizadas:

- **41 pruebas aprobadas:** 19 unitarias, 8 del cliente conectado, 6 de extensiones del protocolo/servidor y 8 originales UDP.
- Servidor sin features gráficas y cliente compilados; CLI help y errores de argumentos comprobados.
- Formato correcto y Clippy de todos los targets con `-D warnings`.
- Dos ventanas gráficas verificadas con resultado 7:0, perspectivas correctas y RTT local observado de 4 ms. Capturas en `docs/evidencias/`.
- Uso de directorios de salida separados dentro de `target/` para conservar los artefactos originales.

El rendimiento bajo carga y la conectividad entre equipos/AWS todavía no están
verificados. Las pruebas UDP necesitaron ejecución fuera del aislamiento de red.

## Continuación

Los antecedentes están en `docs/fases-4-5.md`; la implementación de fase 6,
comandos y prueba de aceptación entre equipos están en `docs/fase-6.md`.
`Ready` vuelve a registrar con un socket nuevo; `Quit` envía LEAVE; PING/PONG
miden RTT. El trabajador repite intenciones cada 50 ms y consume estados autoritativos.

La limpieza de los 106 archivos de `target/` ya versionados sigue pendiente.
El enunciado permite C/Rust para el servidor, pero contiene una nota posterior que menciona solo C; esa contradicción debe aclararse con el docente.

La entrega indicada en `ProyectoPongN1.md` es el **22 de octubre de 2026 a las 23:59**, sin zona horaria explícita. El documento prohíbe commits posteriores a ese límite.
