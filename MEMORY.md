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

### [BETA 1.0] — 2026-10-25

* **Estado:** Actual (MVP Completo y Entregables Finales).
* **Hito:** Consolidación de Fase 6 como MVP jugable de extremo a extremo, preparación de entregables de Fase 7, creación de Makefile, README oficial de la asignatura, guía de distribución y sustentación de código, y saneamiento del repositorio.
* **Alcance:** Por instrucción expresa del usuario ante el cierre inminente de entregas, se consolida la versión BETA (Minimum Viable Product). El juego cuenta con servidor autoritativo UDP Berkeley, cliente gráfico desacoplado por canales MPSC, protocolo binario manual de 10 mensajes con Big-Endian, 41 pruebas automatizadas pasando al 100%, y toda la documentación formal requerida por `ProyectoPongN1.md`.

#### Paso 1. Saneamiento del repositorio Git
* Se eliminaron del seguimiento de git 106 archivos temporales de compilación (`target/`) que se habían versionado accidentalmente en el commit inicial, reduciendo el peso del repo y limpiando `git status`.
* Se actualizó `.gitignore` para omitir `/target`, binarios locales (`server`, `client`) y archivos `.log`.

#### Paso 2. Guía de distribución de código y sustentación oral
* Se creó `docs/distribucion-codigo.md` separando rigurosamente:
  * **Módulo de Red y Concurrencia (Evaluación 100% / Autoría del estudiante):** `src/protocol.rs`, `src/server.rs`, `src/game.rs`, `src/bin/server.rs`, `src/client/network.rs` y `tests/*`.
  * **Módulo de Interfaz Gráfica (Asistido por IA / No computable para nota de redes):** `src/client/renderer.rs`, `src/client/gui.rs`, `src/client/connection.rs`, `src/bin/client.rs`.
  * **Capa de Desacoplamiento:** `NetworkBridge` y contratos `ClientInput` / `GameStateSnapshot`.
* Se añadieron las 5 preguntas y respuestas esenciales para la sustentación oral ante el docente.

#### Paso 3. Creación del Makefile estándar
* Se redactó `Makefile` en la raíz con los targets obligatorios exigidos por la rúbrica: `all`, `server` (compilación headless sin librerías gráficas), `client`, `test` (ejecución de 41 pruebas), `check`, `run-server` (con puertos y archivo de log configurables) y `clean`.

#### Paso 4. Documentación formal en README.md
* Se generó `README.md` estructurado exactamente bajo las cuatro secciones obligatorias de `ProyectoPongN1.md`:
  1. *Introducción:* Contexto, objetivos telemáticos y modelo C/S autoritativo.
  2. *Desarrollo:* Arquitectura detallada, justificación de UDP vs TCP, protocolo binario `MyAppGameProtocol`, concurrencia de partidas, logger dual, instrucciones de compilación/ejecución y guía de despliegue en AWS Academy EC2 con Security Groups.
  3. *Conclusiones:* Análisis de sobrecarga binaria vs JSON, mitigación de pérdida/desorden en UDP, prevención de trampas y medición monotónica de RTT.
  4. *Referencias:* Beej's Guide, RFC 768, Valve Latency Compensation, Rust Networking.

#### Paso 5. Verificación de compilación y pruebas
* Las 41 pruebas pasan al 100% (`cargo test`).
* `cargo check --bin server --no-default-features` y `cargo check --bin client` compilan sin advertencias ni errores.

---

### [Alpha 0.6] — 2026-10-25

* **Estado:** Completado.
* **Hito:** Implementación de fase 6: cliente gráfico conectado al servidor UDP autoritativo, con registro, estados, RTT, abandono y reconexión.
* **Alcance:** Implementación completa solicitada expresamente por el usuario. Se verificaron clientes automatizados y dos ventanas gráficas en un computador. Se conserva Alpha hasta realizar la aceptación formal entre dos equipos físicos; no se declara esa prueba realizada ni se registra despliegue AWS.

#### Paso 1. Sincronizar y verificar la base

* Se consultaron ramas y etiquetas remotas y se ejecutó `git pull --ff-only origin master`: sin novedades, base `b445013` (Alpha 0.5), árbol limpio antes de comenzar.
* Se revisaron las guías, enunciado, memoria, protocolo y contrato de `NetworkBridge`; las 20 pruebas anteriores seguían pasando y ambos binarios compilaban.

#### Paso 2. Definir la configuración e inicio online

* Se añadió `NetworkConfig` con servidor, nickname y correo, validación de puerto, límites de bytes UTF-8 y perfil. La validación no abre sockets ni resuelve DNS en la GUI.
* El cliente inicia online con tres argumentos o muestra un formulario al ejecutarlo sin argumentos. `--demo` conserva la simulación local y `--help` muestra el uso.
* El formulario permite editar los campos y presenta errores de validación. Los errores de argumentos se detectan antes de abrir una ventana.

#### Paso 3. Implementar el trabajador UDP mediante el puente

* Se creó `src/client/network.rs`; el trabajador es dueño exclusivo del socket no bloqueante y comunica entradas/snapshots con la GUI por los canales existentes.
* `spawn_udp_network_worker` devuelve un `JoinHandle` dentro de un resultado; los errores de creación se pueden mostrar sin bloquear el renderizado.
* El socket UDP conectado al endpoint del servidor filtra el origen. El trabajador valida además sesión y paquetes mediante el codec compartido, manteniendo sockets y parsing fuera de `gui.rs` y `renderer.rs`.

#### Paso 4. Registrar, emparejar y mantener las intenciones

* Se implementaron estados de conexión, registro con `HELLO_REQ`/`HELLO_ACK`, lobby e inicio con lado y nickname del rival.
* El saludo se reintenta cada 500 ms; en partida se recuerda y repite la intención actual cada 50 ms, incluido `Stop`, aunque la GUI solo envíe los cambios de tecla.
* Se mantiene un contador modular para comandos entrantes del cliente y se descartan duplicados o estados antiguos recibidos del servidor.
* El timeout inicial de registro es de 5 segundos; tras registrarse, 3 segundos sin respuesta válida producen una desconexión visible y recuperable.

#### Paso 5. Conectar estados autoritativos con la representación visual

* Se mapean coordenadas, palas, marcador y estado del protocolo a `GameStateSnapshot`; los goles `u8` se convierten a `u16`.
* Se conservan lado y nombres del emparejamiento. Las velocidades visuales son estimaciones entre fotos, no una simulación física del cliente; el reinicio de un punto usa el temporizador visual de 800 ms.
* Un `MATCH_START` repetido y un `WAITING_OPPONENT` atrasado no borran el marcador ni reinician una partida.
* Se corrigió el puente para conservar `GameOver` o una razón de desconexión al cerrar el canal, en lugar de reemplazarlos por un error genérico.

#### Paso 6. Añadir RTT y salida voluntaria al protocolo binario

* Se añadieron `PING` (`0x0008`) y `PONG` (`0x0009`), con token `u32` Big-Endian de cuatro bytes: 16 bytes por datagrama completo. El cliente envía ping cada segundo y solo mide RTT si el eco coincide con un token pendiente.
* Se añadió `LEAVE` (`0x000A`), payload vacío y 12 bytes totales, para salida desde lobby o partida.
* Los formatos anteriores y versión 1 se conservan. Las extensiones se documentan en `docs/protocolo.md`; cliente y servidor actualizados usan diez tipos de mensaje.
* `INPUT_CMD`, `PING` y `LEAVE` comparten validación de origen, sesión y secuencia. Ping mantiene señal de vida sin renovar movimiento; Leave en partida utiliza una señal atómica por lado independiente de la cola acotada, para no perder el abandono si la cola está llena.

#### Paso 7. Resolver cierre, reconexión y orden de mensajes terminales

* `Quit` envía Leave como mejor esfuerzo y termina el trabajador. ESC y cierre nativo de ventana generan ese comando. Si la red pierde la salida, el timeout del servidor sigue siendo respaldo.
* `Ready` tras desconexión o fin abre un socket nuevo, limpia sesión/secuencias y vuelve al registro, sin esperar la retención del endpoint anterior. En partida activa no altera la simulación.
* Una revisión independiente detectó que `GAME_OVER` podía adelantarse a `MATCH_START` o a la foto final: se corrigió el cliente para completar nombres/lado y el marcador terminal sin permitir volver a `InMatch`.
* El servidor conserva metadata y foto final por puntuación, reenvía esa foto con el resultado durante cinco segundos y recupera metadata ante saludo repetido. Estas consultas no prolongan la retención. El abandono no inventa un marcador final.
* En la revisión previa al commit se corrigió otro caso de desorden: si llega un abandono antes de `MATCH_START`, se conserva el marcador y las coordenadas de la foto autoritativa pendiente. Una prueba reproduce el marcador 5:2 y verifica que los metadatos tardíos no cambien el ganador ni reabran la partida.

#### Paso 8. Ajustar la GUI para operación online

* Se actualizó el arranque para usar el trabajador real; la demo requiere `--demo`.
* El HUD distingue conexión, demo y RTT pendiente/medido; controles y telemetría usan líneas separadas. Los textos largos se ajustan al espacio disponible.
* Se conserva la renderización desacoplada. El cierre espera el hilo de forma asíncrona hasta un segundo para no congelarse si el DNS del sistema tarda en responder.

#### Paso 9. Verificar con pruebas y ventanas reales

* **41 pruebas aprobadas:** 19 unitarias, 8 en `tests/client_network.rs`, 6 en `tests/protocol_extensions.rs` y las 8 originales de `tests/server_udp.rs`.
* Las pruebas del cliente incluyen cuatro trabajadores/dos partidas, intención sostenida, RTT correlacionado, Quit/Ready, pérdida de registro, mensajes desordenados, sesiones incorrectas, timeout y conservación del resultado.
* Se ejecutó una partida autoritativa completa hasta **7:0** con dos trabajadores reales, sin modificar las reglas del motor para acelerar la victoria. Ambos conservaron el mismo marcador y ganador.
* Se abrieron dos clientes gráficos contra un servidor local: ambas ventanas mostraron **7:0**, nombres y lados correctos y RTT observado de **4 ms**. Se revisó además el formulario y su mensaje de validación.
* Las capturas se guardaron en `docs/evidencias/fase6-ana.png` y `docs/evidencias/fase6-bob.png`. Las ventanas y el servidor temporal se cerraron tras verificar.
* `cargo fmt --all -- --check` y Clippy de todos los targets con `-D warnings` terminaron correctamente. Se comprobaron los binarios, ayuda/errores CLI y el servidor headless; el entorno requiere ejecución fuera del aislamiento para UDP local.

#### Paso 10. Documentar el alcance y la aceptación pendiente

* Se creó `docs/fase-6.md` con arquitectura, comandos de servidor y dos clientes, controles, pruebas y procedimiento entre dos computadores.
* Se actualizaron `docs/protocolo.md`, `docs/estado-actual.md` y `AGENTS.md` para reflejar la implementación real y las extensiones. La guía ya no pide crear un trabajador que existe.
* La verificación en dos equipos físicos, rendimiento bajo carga, RTT por Internet y AWS permanecen pendientes. Las pruebas locales y capturas no se presentan como evidencia de esas condiciones.
* Tras completar la implementación, el usuario solicitó expresamente un commit local con verificación previa. El registro conserva Alpha 0.6 y no incluye un push. El límite del 22 de octubre en el enunciado permanece sin cambios.

* **Archivos intervenidos:**
  * `src/client/network.rs`, `src/client/connection.rs`, `src/client/mod.rs`: trabajador, configuración, formulario y exports.
  * `src/bin/client.rs`, `src/client/gui.rs`, `src/client/renderer.rs`: arranque online, cierre y visualización.
  * `src/client/bridge.rs`, `src/client/types.rs`: preservación de estados terminales y ajustes de documentación/formato.
  * `src/protocol.rs`, `src/server.rs`: ping/pong, salida, validación y recuperación de datos finales.
  * `tests/client_network.rs`, `tests/protocol_extensions.rs`: pruebas nuevas del cliente y extensiones.
  * `docs/fase-6.md`, `docs/protocolo.md`, `docs/estado-actual.md`, `docs/evidencias/`, `AGENTS.md` y `MEMORY.md`: continuidad y evidencias.

* **Próximos pasos pendientes:**
  * Ejecutar el procedimiento de aceptación de fase 6 en dos equipos físicos y registrar marcadores, movimiento, desconexión y RTT de esa red.
  * Una vez acreditado ese criterio, cerrar formalmente fase 6 y revisar la transición a BETA.
  * Fase 7: AWS Academy, pruebas por Internet, README final, Makefile y entrega; la limpieza de los artefactos ya versionados en `target/` sigue pendiente.

---

### [Alpha 0.5] — 2026-10-19

* **Estado:** Completado.
* **Hito:** Fases 4 y 5 implementadas y verificadas: protocolo binario manual, servidor autoritativo y partidas concurrentes independientes.
* **Alcance:** Implementación realizada por solicitud expresa del usuario, primero fase 4 y después fase 5. La memoria se actualiza posteriormente, con autorización del usuario, para dejar constancia del proceso. El cliente gráfico sigue en demo local: no se declara BETA ni MVP online.

#### Paso 1. Revisar el punto de partida y las guías

* Se revisaron `AGENTS.md`, `ProyectoPongN1.md`, esta bitácora, `docs/protocolo.md` y el código existente.
* La base local era `d84d32e`, rama `master` siguiendo `origin/master`: receptor UDP con logs y GUI desacoplada mediante `NetworkBridge`, todavía sin protocolo implementado ni partidas de servidor.
* Se preservó la separación de responsabilidades: sin sockets ni parsing dentro de la GUI y con compilación del servidor sin dependencias gráficas.

#### Paso 2. Resolver la compatibilidad del compilador

* Rust/Cargo 1.82.0 no admitía la edición 2024 de `Cargo.toml`.
* Se instaló Rust 1.85.1 junto al existente y con él se verificó inicialmente el protocolo y el servidor.
* Al comprobar la GUI se detectó que `fontdue 0.9.4`, ya fijado por el lockfile original, utiliza funciones no disponibles en 1.85.1.
* Se instaló Rust **1.90.0** y se fijó para este repositorio en `rust-toolchain.toml`, con `rustfmt` y Clippy. Se conservó la toolchain global predeterminada y no se cambiaron dependencias ni `Cargo.lock`.

#### Paso 3. Implementar la cabecera y los siete mensajes — Fase 4

* Se creó `src/protocol.rs` y se expuso desde `src/lib.rs`.
* Se definieron `Header`, `Packet`, los tipos de mensaje y sus payloads: `HELLO_REQ`, `HELLO_ACK`, `WAITING_OPPONENT`, `MATCH_START`, `INPUT_CMD`, `STATE_SYNC` y `GAME_OVER`.
* La conversión es manual mediante slices, `to_be_bytes`, `from_be_bytes` y representación binaria de `f32`, sin Serde ni serializadores automáticos.
* Se respetaron cabecera de **12 bytes**, entrada de **13 bytes** y estado de **31 bytes**. Las cadenas usan longitud prefijada en bytes UTF-8, con límite de 255 bytes por cadena.

#### Paso 4. Validar y consolidar el protocolo antes de pasar a la fase 5

* El lector comprueba límites antes de extraer bytes; rechaza truncamientos, magic o versión incorrectos, opcodes desconocidos, longitudes incompatibles, bytes sobrantes, UTF-8 inválido y discriminantes inválidos.
* Se rechazan cadenas vacías, asignación de sesión cero y coordenadas no finitas o fuera del campo.
* Se implementó comparación modular de secuencias, incluido el paso `u32::MAX -> 0`, descartando entradas repetidas o atrasadas.
* Pasaron **7 pruebas de protocolo** antes de implementar la fase 5: bytes esperados, ida/vuelta de los siete mensajes, layout de estado, cadenas, errores de formato, coordenadas y secuencias.

#### Paso 5. Crear la simulación autoritativa pura — Fase 5

* Se creó `src/game.rs`, independiente de sockets, relojes del sistema y GUI.
* Cada `Game` posee posiciones, velocidades, intenciones, marcadores y ganador de una única partida.
* Se implementaron movimiento de palas limitado a centros Y `[45, 555]`, rebotes de pelota contra paredes y palas, goles, reinicio al centro, pausa de **800 ms** y victoria a **7 puntos**.
* La simulación usa subpasos de hasta 8 ms y limita la velocidad horizontal tras rebotes. Solo el servidor determina el tiempo y el resultado; los clientes envían intenciones.
* Se añadieron **5 pruebas del motor** para límites y detención, rebotes, goles/pausa/victoria y aislamiento entre instancias.

#### Paso 6. Registrar sesiones y emparejar jugadores

* Se creó `src/server.rs` con `PongServer` y `ServerConfig`, usando `std::net::UdpSocket`.
* El receptor central valida paquetes, asigna IDs no nulos ligados a IP:puerto y administra una cola FIFO de espera.
* Dos registros forman una pareja: izquierda para el primero, derecha para el segundo, con notificación del nickname rival.
* Repetir `HELLO_REQ` con el mismo endpoint y perfil recupera la sesión y su estado; no crea otra sesión ni reinicia una partida.
* Las entradas bajo una sesión ajena, mensajes exclusivos del servidor y secuencias duplicadas o antiguas se rechazan.
* Se limita la capacidad predeterminada a **256 sesiones**, incluyendo las que retienen un resultado final.

#### Paso 7. Ejecutar partidas concurrentes y registrar el tráfico

* Cada pareja se ejecuta en un hilo con estado propio y un canal de entradas acotado a **128** elementos. No se comparte el estado mutable de todas las partidas mediante un candado global.
* Se comparte el socket de envío y se sincroniza el logger con `Mutex` para mantener líneas completas en consola y archivo.
* Se envían estados autoritativos con cadencia objetivo de **60 Hz**, dependiente del host. El planificador conserva los plazos y omite ticks excesivamente atrasados.
* El receptor procesa como máximo **64 datagramas por vuelta** para permitir limpieza y cierre aun con tráfico continuo.
* Se registran RX/TX, origen o destino, sesión, tipo, bytes y eventos de registro, emparejamiento, gol, fin, rechazo y error.
* Se sustituyó el receptor inicial de `src/bin/server.rs` por el arranque de `PongServer`, manteniendo `server <PUERTO> <Archivo Log>` y añadiendo validación del puerto y códigos de salida de error.

#### Paso 8. Manejar pérdidas, desconexiones y liberación de recursos

* Una intención sin entrada fresca expira a los **250 ms** y detiene la pala. El futuro trabajador UDP debe repetir la intención actual, incluido `Stop`, cada **100 ms** como máximo.
* La sesión de lobby o jugador en partida vence a los **10 segundos** sin actividad válida. Una desconexión termina esa partida con `GAME_OVER` y deja continuar a las otras.
* `MATCH_START` se repite cada **500 ms** para recuperar pérdidas del mensaje de control.
* Al terminar se une el hilo y se conserva/repite `GAME_OVER` durante **5 segundos**; después se eliminan las sesiones y se libera capacidad para nuevos registros.
* El receptor utiliza un buffer de UDP completo y descarta datagramas mayores al máximo válido de **524 bytes**, evitando aceptar un prefijo truncado como paquete completo.
* `PongServer::run` permite parada mediante bandera compartida para cerrar y unir los hilos desde las pruebas o un host que lo integre.

#### Paso 9. Verificar el resultado con sockets reales y herramientas de Rust

* Resultado final: **20 pruebas aprobadas** — 7 de protocolo, 5 de simulación y **8 de integración UDP**.
* Las pruebas UDP comprueban: cuatro clientes formando dos parejas con movimientos independientes; registro repetido; sesiones falsas/orden/mensajes inválidos; pérdida de `Stop`; timeout de una pareja sin interrumpir la otra; expiración de lobby; rechazo de datagramas malformados o grandes; y recuperación de capacidad con nuevo registro desde el mismo endpoint.
* Se comprobó la compilación del cliente y se construyó el servidor sin features gráficas. Clippy de la biblioteca headless, servidor y pruebas terminó sin advertencias con `-D warnings`.
* Se verificaron los errores del ejecutable ante argumentos incompletos y puerto inválido. Se aplicó `rustfmt` y `git diff --check` no encontró errores de whitespace.
* Las pruebas de localhost requirieron ejecución fuera del aislamiento de red. No se verificaron todavía la GUI en ejecución, FPS, rendimiento bajo carga ni conectividad entre computadores o AWS.
* Los artefactos de verificación se guardaron en subdirectorios separados de `target/`, conservando los archivos antiguos ya versionados.

Comandos de referencia, desde la raíz del repositorio:

```powershell
cargo test --locked --no-default-features --target-dir target/codex-validation
cargo check --locked --bin server --no-default-features --target-dir target/codex-validation
cargo check --locked --bin client --target-dir target/codex-client-validation
cargo build --locked --bin server --no-default-features --target-dir target/codex-validation
cargo clippy --locked --no-default-features --all-targets --target-dir target/codex-validation -- -D warnings
```

#### Paso 10. Dejar la documentación preparada para continuar

* Se completó `docs/protocolo.md` con payload vacío de `WAITING_OPPONENT`, procedimientos de registro, secuencias, reintentos, expiración, fin y validación.
* Se creó `docs/fases-4-5.md` con la explicación de arquitectura, pasos de implementación, pruebas y ejecución.
* Se actualizó `docs/estado-actual.md` y se marcaron fases 4 y 5 completadas y **fase 6 actual** en `AGENTS.md`; también se ajustaron las referencias de rama a `master` y los nombres reales del protocolo.
* El registro describe cambios locales: no implica commit, push ni despliegue.

* **Archivos intervenidos en el hito:**
  * `src/protocol.rs`: codec manual, tipos y siete pruebas.
  * `src/game.rs`: simulación autoritativa y cinco pruebas.
  * `src/server.rs`: registro, emparejamiento, hilos, estados, logging y limpieza.
  * `src/lib.rs`: exposición de los módulos nuevos.
  * `src/bin/server.rs`: arranque y validación CLI.
  * `tests/server_udp.rs`: ocho pruebas con sockets reales.
  * `rust-toolchain.toml`: Rust 1.90.0 para el repositorio.
  * `docs/protocolo.md`, `docs/fases-4-5.md`, `docs/estado-actual.md` y `AGENTS.md`: especificación y continuidad.
  * `MEMORY.md`: registro posterior del hito como Alpha 0.5.

* **Próximos pasos pendientes:**
  * **Fase 6:** implementar el trabajador UDP del cliente y sustituir `spawn_offline_demo_worker`, usando exclusivamente `NetworkBridge` para comunicar GUI y red.
  * Integrar reintentos, repetición de intenciones, descarte de estados atrasados y mapeo de los datos del protocolo a `GameStateSnapshot`.
  * Decidir la representación de `Ready`, `Quit` y RTT, que no tienen mensajes definidos en el catálogo actual, y de los campos visuales no incluidos en `STATE_SYNC`.
  * Verificar una partida completa entre dos clientes gráficos reales antes de declarar BETA.
  * **Fase 7:** desplegar en AWS Academy, probar por Internet y preparar README, Makefile y entrega. Sigue pendiente limpiar del índice los artefactos antiguos de `target/` y aclarar con el docente la nota contradictoria C/Rust del enunciado.

---

### Revisión inicial del entorno local — 2026-10-19 (diagnóstico histórico, superado por Alpha 0.5)

* **Versión funcional:** Se mantiene Alpha 0.4; no se implementaron componentes nuevos.
* **Sincronización:** Repositorio descargado en `C:\Users\Asus\Documents\ChatGPT\Pong0`, rama `master` siguiendo `origin/master`, commit `d84d32e`.
* **Diagnóstico:** Código y documentación revisados; la fase activa sigue siendo la 4. La GUI utiliza simulación local y el servidor solo recibe y registra datagramas.
* **Validación:** Ambos `cargo check` (servidor sin features y cliente) fallaron al analizar el manifiesto porque Cargo 1.82.0 no admite la edición 2024. No se validó compilación ni ejecución en este equipo.
* **Informe:** `docs/estado-actual.md` contiene evidencias, fases pendientes y precisiones de protocolo, entrega y repositorio.
* **Siguiente paso:** Resolver la compatibilidad de la toolchain y retomar la cabecera binaria de 12 bytes, manteniendo la metodología pedagógica de AGENTS.md.

---

### [Alpha 0.4] — 2026-10-17
* **Estado:** Completado
* **Hito:** Implementación del subsistema de logging dúplex con marcas de tiempo y bucle de eventos UDP continuo en el servidor (`PongServer`).
* **Descripción técnica:**
  * Consolidación del bucle continuo de escucha UDP (`loop { socket.recv_from(&mut buf) }`) en `src/bin/server.rs`, transformando el servidor en un servicio residente permanente.
  * Implementación de la función de logging dúplex `registrar_log(&mut File, &str)` con escritura simultánea a consola estándar (`println!`) y persistencia atómica en disco (`writeln!`).
  * Apertura robusta del archivo de trazas mediante `OpenOptions::new().create(true).append(true)` garantizando que los logs no se sobreescriban entre reinicios del servicio.
  * Integración de marcas de tiempo universales calculadas mediante `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()` para auditoría de eventos y datagramas entrantes.
  * Higiene de memoria mediante reasignación del buffer `[0u8; 1024]` en cada vuelta de ciclo para prevenir contaminación entre datagramas.
  * Verificación funcional exitosa en tiempo real mediante inyección de paquetes UDP de prueba y comprobación de persistencia física en archivo de log.
  * Blindaje pedagógico formalizado en `AGENTS.md` (Reglas 1, 2 y Sección 5) estableciendo avance atómico, directivas conceptuales y ejemplos de juguete aislados.
* **Archivos intervenidos:**
  * `src/bin/server.rs`: Lógica del bucle continuo, manejo de archivo de log y timestamps.
  * `AGENTS.md`: Metodología pedagógica paso a paso y protocolo de interacción.
  * `MEMORY.md`: Registro de versión Alpha 0.4.
* **Próximos pasos pendientes:**
  * Diseñar el módulo de serialización y deserialización binaria manual (`src/protocol.rs`) para la cabecera fija de 12 bytes (Fase 4).
  * Implementar pruebas unitarias en Rust para verificar empaquetado y desempaquetado exacto en orden Big-Endian sin uso de Serde.
  * Definir las estructuras en Rust para los payloads de registro (`HELLO_REQ` / `HELLO_ACK`) y movimiento (`INPUT_CMD`).

---

### [Alpha 0.3] — 2026-10-14
* **Estado:** Completado
* **Hito:** Especificación formal del protocolo binario de capa de aplicación (`MyAppGameProtocol`) en `docs/protocolo.md`.
* **Descripción técnica:**
  * Diseño de la cabecera fija de 12 bytes: `Magic Byte` (`0x50`), `Protocol Version` (`0x01`), `Message Type` (`u16`), `Sequence Number` (`u32`), `Session ID` (`u16`), `Payload Length` (`u16`).
  * Adopción de Network Byte Order (Big-Endian) para todos los campos multibyte (`u16`, `u32`, `f32`).
  * Definición del catálogo de mensajes: `HELLO_REQ` (`0x0001`), `HELLO_ACK` (`0x0002`), `WAITING_OPPONENT` (`0x0003`), `MATCH_START` (`0x0004`), `INPUT_CMD` (`0x0005`), `STATE_SYNC` (`0x0006`), `GAME_OVER` (`0x0007`).
  * Especificación de empaquetado para cadenas variables (Nickname, Email) mediante codificación *Length-Prefixed* (Pascal string).
  * Optimización de paquetes de alta frecuencia: `INPUT_CMD` (13 bytes totales) y `STATE_SYNC` (31 bytes totales: 16B coords, 2B goles, 1B status, 12B cabecera).
* **Archivos intervenidos:**
  * `docs/protocolo.md`: Documento formal de especificación del protocolo binario.
  * `MEMORY.md`: Registro de versión Alpha 0.3.
  * `AGENTS.md`: Transición a Fase 3 (Comunicación UDP Básica y Logging).
* **Próximos pasos pendientes:**
  * Implementación del subsistema de logging concurrente (archivo + terminal) en el servidor (Fase 3).
  * Bucle de escucha continuo no bloqueante en `src/bin/server.rs`.
  * Serialización/deserialización manual en Rust de cabeceras y payloads (Fase 4).

---

### [Alpha 0.2] — 2026-10-13
* **Estado:** Completado
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
