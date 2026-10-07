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

### [Alpha 0.1] — 2026-10-07
* **Estado:** Actual
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
* **Próximos pasos pendientes:**
  * Ajustar el formateo del socket (`0.0.0.0:<PUERTO>`).
  * Implementar el bucle principal de escucha y no bloqueo o concurrencia.
  * Diseñar la estructura binaria del protocolo `MyAppGameProtocol` y el sistema de logging simultáneo a consola y `<Archivo Log>`.
