# MEMORY.md — Bitácora y Registro de Versiones del Proyecto Pong

Este documento almacena el historial y la memoria técnica de todos los avances realizados en el desarrollo del videojuego multijugador **Pong en Red**.

---

## Criterios del Ciclo de Vida y Versionado

El progreso del proyecto se rige por las siguientes reglas estrictas de transición de versiones:

1. **Fase Alpha (`Alpha 0.1` en adelante, incrementos de `+0.1`):**
   * Corresponde a la etapa de desarrollo iterativo temprano y medio.
   * Abarca el diseño de arquitectura, sockets Berkeley UDP, definición y serialización del protocolo binario (`MyAppGameProtocol`), subsistema de logs, manejo de concurrencia y física elemental.
   * La versión asciende de 0.1 en 0.1 (`Alpha 0.1` -> `Alpha 0.2` -> `Alpha 0.3`...).

2. **Fase BETA (`BETA` / `Beta 0.x`):**
   * **Condición estricta de transición:** Se alcanza **ÚNICA Y EXCLUSIVAMENTE** cuando el juego Pong sea jugable y pueda considerarse un Producto Mínimo Viable (**MVP**).
   * Requiere: Servidor autoritativo enlazado, clientes capaces de conectarse y emparejarse, transmisión de entradas de palas, cálculo de físicas en servidor y sincronización bidireccional de estados donde sea posible disputar una partida real.

3. **Fase v1 (`v1.0`):**
   * **Condición de transición:** Se alcanza cuando la interfaz de usuario esté completa, refinada y el juego sea totalmente jugable, robusto y preparado para su despliegue y entrega final según los requerimientos estipulados en `ProyectoPongN1.md`.

---

## Historial de Versiones

### [Alpha 0.2] — 2026-10-13
* **Estado:** Actual
* **Hito:** Construcción de la interfaz gráfica funcional (`PongClient` GUI) con renderizado a 60 FPS y puente de red desacoplado (`NetworkBridge`).
* **Descripción técnica:**
  * Reestructuración del proyecto en Cargo con soporte multibinario independiente: `server` (`src/bin/server.rs`) y `client` (`src/bin/client.rs`).
  * Configuración de dependencias opcionales (`macroquad = "0.4.16"` bajo la feature `client`) para asegurar que el binario del servidor compile de forma limpia e instantánea en entornos headless en la nube (AWS Academy EC2) sin requerir bibliotecas gráficas del sistema.
  * Implementación de la capa gráfica del cliente con resolución virtual fija de 800x600 px y adaptación automática de relación de aspecto.
  * Estética retro arcade completa: campo delimitado, red punteada vertical, palas en neón Cian (P1) y Naranja (P2), pelota brillante, marcador digital superior y overlays de estado de partida (`Connecting`, `WaitingForOpponent`, `PointScored`, `GameOver`, etc.).
  * Diseño del contrato desacoplado de comunicación entre hilos mediante `NetworkBridge` y canales MPSC (`std::sync::mpsc`):
    * `ClientInput`: Comandos emitidos por el GUI (`MoveUp`, `MoveDown`, `Stop`, `Ready`, `Quit`).
    * `GameStateSnapshot`: Instantáneas autoritativas recibidas desde la red y consumidas sin bloqueo con `poll_latest_state()`.
  * Integración de un trabajador local (`spawn_offline_demo_worker`) que permite ejecutar y probar interactivamente el cliente gráfico de forma inmediata.
  * Documentación exhaustiva en `AGENTS.md` (Sección 7) del diagrama de flujo de datos y la guía paso a paso para la futura conexión de sockets Berkeley UDP.
* **Archivos intervenidos:**
  * `Cargo.toml`: Configuración de biblioteca `pong_lib`, binarios `server`/`client` y feature opcional `client`.
  * `src/bin/server.rs`: Migración del servidor de escucha UDP.
  * `src/bin/client.rs`: Punto de entrada de la aplicación gráfica del cliente.
  * `src/client/types.rs`: Definición de enums y structs de contrato (`ClientInput`, `GameStateSnapshot`, `MatchStatus`, `PlayerSide`).
  * `src/client/bridge.rs`: Puente `NetworkBridge` y simulador demo local.
  * `src/client/renderer.rs`: Motor de dibujado con Macroquad y HUD de telemetría.
  * `src/client/gui.rs`: Bucle reactivo de renderizado y captura de teclado.
  * `src/client/mod.rs` y `src/lib.rs`: Exposición limpia de módulos.
  * `AGENTS.md`: Registro de especificaciones de integración de red y excepciones metodológicas.
* **Próximos pasos pendientes:**
  * Diseñar la estructura binaria de cabeceras y payloads de `MyAppGameProtocol` (Fase 2).
  * Implementar el bucle de recepción continua no bloqueante del servidor e hilo de logs concurrentes (Fase 3).
  * Crear la función `spawn_udp_network_worker` en el cliente para reemplazar la demo local por tráfico real sobre sockets Berkeley UDP.

---

### [Alpha 0.1] — 2026-10-07
* **Estado:** Completado
* **Hito:** Inicialización del proyecto, captura de argumentos CLI y socket UDP base.
* **Descripción técnica:**
  * Configuración del proyecto base en Rust (`Pong0`, edición 2024).
  * Implementación del parseador preliminar de argumentos por línea de comandos para recibir `<PUERTO>` y `<Archivo Log>`, alineado con la especificación de ejecución `./server <PORT> <Log File>` requerida por `ProyectoPongN1.md`.
  * Verificación básica de cantidad de argumentos para asegurar la invocación correcta del binario.
  * Inicialización y enlace (`bind`) del socket UDP mediante `std::net::UdpSocket` en la dirección local.
  * Declaración del buffer de lectura (`[u8; 1024]`) y llamada a `recv_from` para esperar la recepción de datagramas entrantes.
  * Establecimiento de las directivas operativas y pedagógicas del especialista en redes en `AGENTS.md`.
* **Archivos intervenidos:**
  * `Cargo.toml`: Definición de metadatos del paquete.
  * `src/main.rs`: Lógica de arranque del servidor UDP y argumentos.
  * `AGENTS.md`: Protocolo de actuación, reglas pedagógicas y fuente de verdad en `ProyectoPongN1.md`.
