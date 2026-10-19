# Implementación de las fases 4 y 5

Revisión: 2026-10-19. Estas fases se implementaron secuencialmente por solicitud
expresa del usuario. `MEMORY.md` no se modificó durante esta implementación.

## Paso 1: separar el protocolo del transporte

`src/protocol.rs` representa la cabecera y los siete mensajes definidos en
`docs/protocolo.md`. `Header::encode/decode` y `Packet::encode/decode` convierten
los datos manualmente con slices, `to_be_bytes` y `from_be_bytes`; no usan Serde.

La cabecera ocupa 12 bytes. `INPUT_CMD` ocupa 13 y `STATE_SYNC` 31. Las cadenas
cuentan bytes UTF-8, no caracteres. El lector verifica cada límite antes de
extraer bytes y rechaza datos sobrantes, mensajes desconocidos, valores inválidos
y coordenadas no finitas.

Antes de comenzar la fase 5 pasaron las siete pruebas del protocolo: disposición
binaria exacta, ida/vuelta de los siete mensajes, truncamientos, UTF-8, límites,
valores inválidos y secuencias modulares.

## Paso 2: construir una simulación autoritativa comprobable

`src/game.rs` mantiene el estado de una sola partida. Recibe intenciones de
movimiento, limita las palas al campo, calcula rebotes, anota goles, pausa el
saque 800 ms y termina al llegar a siete puntos. El tiempo lo controla el servidor.

El motor no conoce sockets ni librerías gráficas. Esta separación permite probar
físicas y puntuaciones de forma determinista. Cada instancia posee sus propios
datos; ninguna variable global representa el estado de una partida.

## Paso 3: registrar sesiones y emparejar jugadores

`src/server.rs` recibe UDP con un socket estándar de Berkeley (`UdpSocket`).
Valida los datagramas, asigna sesiones ligadas a IP:puerto y coloca los registros
en una cola FIFO. Un saludo repetido recupera la sesión existente. Dos sesiones
en espera crean una pareja con lados y nombres de oponente asignados.

No se acepta que un cliente envíe coordenadas, marcadores, notificaciones de fin
ni comandos bajo el ID de otro endpoint. Las entradas repetidas o atrasadas se
descartan antes de enviarlas a la simulación.

## Paso 4: ejecutar partidas concurrentes sin compartir su estado

```text
Clientes UDP
     |
     v
Receptor y registro central
     |                    |
Canal acotado         Canal acotado
     |                    |
Hilo partida A        Hilo partida B
Game propio           Game propio
     |                    |
     +---- STATE_SYNC ----+
              |
         Socket UDP compartido
```

Cada pareja tiene un hilo y una cola de entradas de capacidad 128. El hilo es el
único dueño de su `Game`. El socket y el logger se comparten; el archivo se protege
con `Mutex` para evitar líneas mezcladas. No se mantiene un candado global sobre
las físicas de todas las partidas.

El receptor procesa hasta 64 datagramas por vuelta para dar tiempo a la limpieza.
Las colas acotadas permiten descartar exceso de entradas sin bloquear al resto.
La cadencia objetivo es 60 ticks por segundo; el sistema operativo y el coste del
logging pueden reducirla. Los plazos conservan su cadencia sin acumular el retraso
normal de cada despertar, y omiten ticks si el host se retrasa demasiado.

## Paso 5: cerrar partidas y recuperar pérdidas

La intención de movimiento expira a los 250 ms. El futuro trabajador de red del
cliente deberá repetir su intención actual, incluso `Stop`, cada 100 ms como
máximo. La sesión expira a los 10 segundos sin actividad válida.

`MATCH_START` se repite cada 500 ms. Al terminar, el servidor une el hilo y
conserva/repite `GAME_OVER` durante cinco segundos. Después elimina las sesiones
y libera capacidad. Una pareja que termina no interrumpe las otras.

Los detalles de registro, pérdida de paquetes, secuencias y estados terminales
están en las secciones 5 y 6 de `docs/protocolo.md`.

## Paso 6: compilar y verificar

La herramienta original era Rust 1.82.0 y no admitía edición 2024. Se instaló
primero 1.85.1, suficiente para el servidor; la compilación de la GUI descubrió
que `fontdue 0.9.4`, ya fijado por el lockfile original, utiliza funciones más
recientes. Se instaló **1.90.0** y se fijó para este repositorio mediante
`rust-toolchain.toml`, conservando la toolchain global predeterminada y sin
cambiar dependencias ni `Cargo.lock`.

Comandos desde la raíz del repositorio:

```powershell
cargo test --locked --no-default-features --target-dir target/codex-validation
cargo check --locked --bin server --no-default-features --target-dir target/codex-validation
cargo check --locked --bin client --target-dir target/codex-client-validation
cargo clippy --locked --no-default-features --all-targets --target-dir target/codex-validation -- -D warnings
```

Se verificaron **20 pruebas**:

| Grupo | Cantidad | Qué comprueba |
| --- | ---: | --- |
| Protocolo | 7 | Bytes, límites, ida/vuelta, validación y secuencias |
| Simulación | 5 | Límites, rebotes, goles, pausa, victoria y aislamiento |
| UDP real | 8 | Dos parejas, reintentos, sesiones falsas, orden, pérdida de Stop, timeout, datagramas inválidos y liberación de capacidad |

Los sockets de las pruebas usan puertos efímeros en `127.0.0.1`. En este entorno
fue necesario ejecutarlas fuera del aislamiento de red para recibir datagramas.
Los resultados no acreditan todavía conectividad entre computadores ni en AWS.

Para iniciar el servidor:

```powershell
cargo run --locked --bin server --no-default-features --target-dir target/codex-validation -- 8080 target/codex-validation/server.log
```

La interfaz de ejecución sigue siendo `server <PUERTO> <Archivo Log>`.
`PongServer::run` acepta una bandera de parada para cerrar y unir los hilos desde
pruebas o un host que lo integre. El ejecutable permanece activo hasta detenerlo.

## Próxima fase

La fase 6 debe implementar el trabajador UDP del cliente y sustituir la demo local
a través de `NetworkBridge`. La GUI sigue usando `spawn_offline_demo_worker`.
Faltan definir la integración de `Ready`/`Quit`, la medición RTT y comprobar dos
clientes gráficos reales. El despliegue AWS y los entregables finales son fase 7.
