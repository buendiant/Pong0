# Distribución de Responsabilidades y Clasificación del Código

Este documento delimita formalmente la arquitectura del proyecto **Pong Multijugador en Red**, clasificando cada archivo entre los componentes de **Red, Protocolo y Concurrencia** (sujetos a estricta evaluación académica y sustentación oral) y los componentes de **Interfaz Gráfica / GUI** (asistidos por Inteligencia Artificial y no computables para la nota técnica de telemática).

---

## 1. Resumen Ejecutivo de Clasificación

| Archivo | Categoría | Evaluación Docente | Responsabilidad Técnica |
| :--- | :--- | :---: | :--- |
| [`src/protocol.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/protocol.rs) | **Red / Protocolo** | **100% (Vital)** | Especificación y serialización manual binaria de `MyAppGameProtocol` (Big-Endian, sin Serde). |
| [`src/server.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/server.rs) | **Red / Concurrencia** | **100% (Vital)** | Servidor autoritativo, socket Berkeley UDP, matchmaking, $N$ partidas concurrentes y logging dual. |
| [`src/game.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/game.rs) | **Lógica / Simulación** | **100% (Vital)** | Motor de física autoritativa pura (física de pelota, rebotes, colisiones de palas y tanteador). |
| [`src/bin/server.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/bin/server.rs) | **Red / Servidor CLI** | **100% (Vital)** | CLI de inicio (`./server <PORT> <Log File>`) y arranque del demonio headless. |
| [`src/client/network.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/network.rs) | **Red / Cliente UDP** | **100% (Vital)** | Trabajador UDP en segundo plano, retransmisiones, RTT (Ping/Pong), timeouts y recepción de estados. |
| [`tests/*`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/tests/) | **Pruebas de Red** | **100% (Vital)** | Batería de 41 pruebas automatizadas de sockets, concurrencia, pérdida simulada y desorden. |
| [`src/client/bridge.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/bridge.rs) | **Frontera Arquitectónica** | **Diseño (Arquitectura)** | Puente `NetworkBridge` con canales MPSC desacoplados entre la GUI y la Red. |
| [`src/client/types.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/types.rs) | **Frontera Arquitectónica** | **Diseño (Arquitectura)** | Contratos de datos compartidos (`ClientInput`, `GameStateSnapshot`, `MatchStatus`). |
| [`src/client/connection.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/connection.rs) | **Interfaz Gráfica / GUI** | *No evaluado (IA)* | Formulario interactivo en pantalla para captura de IP, puerto, nickname y email. |
| [`src/client/gui.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/gui.rs) | **Interfaz Gráfica / GUI** | *No evaluado (IA)* | Bucle de eventos visuales a 60 FPS y captura de teclado (`W`, `S`, flechas, `ESPACIO`, `ESC`). |
| [`src/client/renderer.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/renderer.rs) | **Interfaz Gráfica / GUI** | *No evaluado (IA)* | Renderizado visual 2D con Macroquad (mesa 800x600, palas, pelota, marcador, overlays, HUD). |
| [`src/bin/client.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/bin/client.rs) | **Interfaz Gráfica / GUI** | *No evaluado (IA)* | Punto de entrada del cliente gráfico y configuración de ventana Macroquad. |

---

## 2. Componentes de Red, Protocolo y Concurrencia (Estudio al Pie de la Letra)

Estos son los archivos sobre los cuales se fundamenta la nota del proyecto y la sustentación oral de Telemática.

### A. [`src/protocol.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/protocol.rs) — Especificación y Codec Binario
* **Objetivo:** Definición formal de la PDU (Protocol Data Unit) a nivel de capa de aplicación (`MyAppGameProtocol`).
* **Conceptos clave que debes dominar:**
  1. **Cabecera Fija de 12 bytes:**
     * `magic` (1 byte, `u8`, constante `0x50` = ASCII 'P'): Identificador que valida que el paquete pertenece a este juego.
     * `version` (1 byte, `u8`, constante `1`): Compatibilidad de versión de capa de aplicación.
     * `msg_type` (2 bytes, `u16` Big-Endian, opcodes `1` a `10`): Identifica el tipo de mensaje (`HelloReq`, `StateSync`, `InputCmd`, `Ping`, etc.).
     * `seq_num` (4 bytes, `u32` Big-Endian): Número de secuencia para ordenamiento, detección de duplicados y descarte de estados viejos.
     * `session_id` (2 bytes, `u16` Big-Endian): Token asignado al cliente por el servidor tras el handshake (`0` durante el saludo inicial).
     * `payload_len` (2 bytes, `u16` Big-Endian): Longitud exacta en bytes del cuerpo que acompaña a la cabecera.
     *(Suma total: 1 + 1 + 2 + 4 + 2 + 2 = 12 bytes exactos en Network Byte Order).*
  2. **Serialización Manual sin Serde:**
     * Se utilizan métodos estándar de bajo nivel: `.to_be_bytes()` para empaquetar enteros y `from_be_bytes()` para desempaquetar desde slices de bytes `&[u8]`.
     * Las cadenas de texto (`nickname`, `email`) se transmiten con longitud prefijada (Pascal-style): 1 byte de longitud seguido de los bytes UTF-8 exactos, previniendo desbordamientos de búfer.
     * Las coordenadas y velocidades son números de punto flotante de 32 bits (`f32`), convertidos a bytes mediante `f32::to_bits()` en Big-Endian.
  3. **Aritmética Modular de Secuencias:**
     * Función que detecta si un paquete entrante es más reciente que el último procesado, manejando correctamente el desbordamiento (*wrap-around*) de los contadores enteros.

### B. [`src/server.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/server.rs) — Servidor Autoritativo y Concurrencia
* **Objetivo:** Orquestar el ciclo de vida de la red, aceptar datagramas UDP, emparejar jugadores y mantener partidas independientes concurrentes.
* **Conceptos clave que debes dominar:**
  1. **Sockets Berkeley UDP:**
     * Uso de `std::net::UdpSocket` configurado en modo no bloqueante (`set_nonblocking(true)`).
     * Bucle de eventos principal (`recv_from`): recibe datagramas de cualquier cliente en el puerto configurado.
  2. **Concurrencia de $N$ Partidas Simultáneas:**
     * Cola de emparejamiento (*matchmaking queue*): cuando un cliente se registra (`HELLO_REQ`), el servidor responde `HELLO_ACK` y lo encola. Al llegar un segundo cliente, se extrae la pareja, se les asigna lado (Left = P1, Right = P2) y se lanza un hilo de ejecución independiente (`std::thread::spawn`) para simular su partida.
     * Aislamiento total: la partida de la pareja A corre en su propio tick-rate (ej. 60 Hz) y sus eventos no bloquean ni interfieren con la partida de la pareja B.
  3. **Manejo de Pérdida y Desorden de UDP:**
     * Idempotencia: si un cliente reenvía `HELLO_REQ` porque se perdió el ack, el servidor responde con el estado actual sin duplicar la sesión en la cola.
     * Retención de estados finales: al terminar una partida (`GAME_OVER`), el resultado se retiene en memoria durante 5 segundos para retransmitirlo si el cliente perdió el datagrama de victoria/derrota.
  4. **Subsistema de Logging Dual (`Logger`):**
     * Cada datagrama entrante y saliente se formatea con marca de tiempo UTC/Local, dirección `IP:Puerto`, tamaño en bytes y tipo de mensaje.
     * Se escribe en simultáneo a la consola estándar (`stdout`) y al archivo indicado por parámetro de línea de comandos (`<Log File>`).

### C. [`src/game.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/game.rs) — Motor de Física Autoritativa
* **Objetivo:** Ser la única fuente de verdad (*Single Source of Truth*) para la simulación del juego.
* **Conceptos clave que debes dominar:**
  1. **Modelo Autoritativo:**
     * Los clientes **nunca** deciden dónde está la pelota ni si hubo gol. Los clientes únicamente envían intenciones ("presioné arriba" o "presioné abajo").
     * El servidor aplica los movimientos de las palas dentro de los límites del campo (mesa de 800x600 virtual), avanza la posición de la pelota en base a su vector velocidad $(v_x, v_y)$, calcula el rebote elástico con las palas y paredes superior/inferior, y detecta cuando la pelota cruza la línea de meta.
  2. **Paso de Simulación Discreto:**
     * Frecuencia fija ($\Delta t$). La pelota se reinicia en el centro con un temporizador tras cada anotación antes de reanudar el juego.

### D. [`src/client/network.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/network.rs) — Trabajador UDP del Cliente
* **Objetivo:** Administrar la conexión UDP en el cliente sin bloquear la tasa de refresco gráfica.
* **Conceptos clave que debes dominar:**
  1. **Hilo de Red Dedicado:**
     * El socket UDP del cliente vive en un hilo separado instanciado por `spawn_udp_network_worker`.
  2. **Retransmisión de Intenciones:**
     * En UDP, un paquete de entrada puede perderse. El trabajador mantiene la última intención del usuario (`MoveUp`, `MoveDown`, `Stop`) y la retransmite periódicamente cada 50 ms junto con el número de secuencia correspondiente para garantizar que el servidor siempre conozca la acción actual del jugador.
  3. **Medición de RTT (Ping / Pong):**
     * Cada segundo, el cliente envía un mensaje `PING` con un token aleatorio `u32` y registra la marca de tiempo local de alta resolución (`Instant::now()`).
     * Al recibir el `PONG` con el mismo token, calcula la diferencia de tiempo: $\text{RTT} = t_{\text{actual}} - t_{\text{envío}}$.
     * Esto permite medir latencia bidireccional sin necesidad de sincronización de relojes entre cliente y servidor.

### E. [`tests/*`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/tests/) — Pruebas de Integración y Estrés
* **Conceptos evaluados:**
  * [`tests/server_udp.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/tests/server_udp.rs): Comprueba sockets reales en loopback, múltiples clientes conectándose en paralelo y verificación de que el cierre de una partida no interrumpe a las demás.
  * [`tests/client_network.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/tests/client_network.rs): Simula pérdida de paquetes, llegada fuera de orden de estados y partidas completas automatizadas hasta 7 goles.

---

## 3. Componentes de Interfaz Gráfica / GUI (Asistidos por IA)

Estos archivos se encargan de la presentación visual y la interacción con el usuario en el cliente. De acuerdo con las instrucciones docentes, **no se evalúan en la rúbrica de redes** y se autorizó su generación mediante herramientas de IA.

1. **[`src/client/renderer.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/renderer.rs):**
   * Dibuja en la ventana mediante la librería gráfica `macroquad`.
   * Proyecta coordenadas virtuales fijas de $800 \times 600$ píxeles al tamaño real de la ventana.
   * Dibuja la mesa con línea central punteada, las palas de ambos jugadores, la pelota, los marcadores numéricos, textos de estado ("Buscando oponente...", "Partida en curso") y la telemetría en el HUD (FPS y latencia RTT).
2. **[`src/client/gui.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/gui.rs):**
   * Controla el ciclo de dibujado a 60 fotogramas por segundo (FPS).
   * Monitorea las pulsaciones del teclado (`is_key_down(KeyCode::W)`, etc.) y envía eventos al canal `input_sender`.
   * Si no hay red activa, permite la visualización de la demo offline (`--demo`).
3. **[`src/client/connection.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/connection.rs):**
   * Renderiza el formulario visual cuando se abre el cliente sin parámetros CLI.
   * Maneja cajas de texto interactivas para Servidor (`IP:Puerto`), Nickname y Correo electrónico, con validación de caracteres antes de iniciar la conexión.
4. **[`src/bin/client.rs`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/bin/client.rs):**
   * Función `main()` del cliente anotada con `#[macroquad::main("Pong")]`.
   * Configura la ventana gráfica (ancho, alto, título) y analiza los argumentos de consola.

---

## 4. La Frontera de Desacoplamiento: [`NetworkBridge`](file:///c:/Users/jabs1/Documents/PongTelematica/Pong0/src/client/bridge.rs)

El elemento arquitectónico que conecta ambos mundos de forma limpia es el **Puente de Red** (`NetworkBridge`).

```text
┌──────────────────────────────────────────────┐
│        HILO DE INTERFAZ GRÁFICA (GUI)        │
│             (Asistido por IA)                │
│                                              │
│   - Bucle a 60 FPS (macroquad)               │
│   - Lectura de teclado (W, S, Flechas)       │
│   - Renderizado de palas, pelota, marcador   │
└──────────────┬────────────────▲──────────────┘
               │                │
          ClientInput     GameStateSnapshot
         (Canal MPSC)       (Canal MPSC)
               │                │
┌──────────────▼────────────────┴──────────────┐
│          HILO DE RED UDP (Estudiante)        │
│          (Evaluado al 100% en Redes)         │
│                                              │
│   - Dueño exclusivo del UdpSocket            │
│   - Serialización/deserialización binaria     │
│   - Medición de RTT (Ping/Pong)              │
│   - Retransmisión y timeouts                 │
└──────────────────────┬───────────────────────┘
                       │ Datagramas UDP
                       ▼
            [ SERVIDOR AUTORITATIVO ]
```

### ¿Por qué esta separación es fundamental?
1. **Prevención de congelamiento de pantalla:** Si el hilo de la GUI llamara directamente a operaciones de red (`send_to` o `recv_from`), cualquier variación de latencia, congestión o pérdida de paquetes congelaría los FPS del juego.
2. **Independencia del Servidor:** El servidor de Pong no compila ninguna librería gráfica (`--no-default-features`), lo que le permite ejecutarse de manera ligera en instancias en la nube (AWS EC2 Linux) sin servidores de ventanas (X11/Wayland).

---

## 5. Preguntas Clave para la Sustentación Oral

1. **¿Por qué eligieron UDP (`SOCK_DGRAM`) en lugar de TCP (`SOCK_STREAM`)?**
   * *Respuesta:* En juegos de acción rápida como Pong, la frescura del dato es prioritaria sobre la confiabilidad absoluta. TCP garantiza entrega y orden mediante retransmisiones y control de congestión, lo que provocaría *Head-of-Line Blocking* (un paquete perdido congelaría todos los paquetes posteriores, disparando el lag). En Pong, si se pierde la posición de la pelota del milisegundo 10, no interesa recuperarla: lo que interesa es recibir inmediatamente la posición del milisegundo 20.
2. **¿Cómo controlaron el desorden o duplicados de paquetes en UDP?**
   * *Respuesta:* Cada datagrama de `MyAppGameProtocol` incluye un número de secuencia entero (`seq_num`). El receptor mantiene un registro del último número de secuencia procesado y descarta cualquier paquete cuyo número de secuencia sea menor o igual (usando aritmética modular para admitir el desbordamiento natural de enteros).
3. **¿Cómo garantizan que un jugador no altere su cliente para ganar?**
   * *Respuesta:* Mediante una arquitectura **Cliente-Servidor Autoritativo**. El cliente no tiene la capacidad de decidir la trayectoria de la pelota, los rebotes ni los puntos; únicamente reporta sus intenciones de entrada (`MoveUp`, `MoveDown`, `Stop`). El servidor calcula la física y emite instantáneas autoritativas (`STATE_SYNC`).
4. **¿Por qué serializaron manualmente en binario en lugar de usar JSON?**
   * *Respuesta:* Un paquete de estado en JSON ocupa entre 150 y 250 bytes en texto plano, requiere parseo intensivo de strings y fragmenta memoria. Con `MyAppGameProtocol` binario, la cabecera ocupa exactamente 12 bytes y el payload de estado 31 bytes (43 bytes en total), viajando en orden de red Big-Endian (`to_be_bytes()`), optimizando drásticamente el uso de ancho de banda y la velocidad de procesamiento en CPU.
5. **¿Cómo miden el RTT en tiempo real?**
   * *Respuesta:* El cliente genera periódicamente un mensaje `PING` con un identificador o token `u32` y guarda en memoria el instante exacto (`Instant::now()`). El servidor recibe el ping y responde de inmediato con un mensaje `PONG` reflejando el mismo token. Al coincidir el token en el cliente, calculamos el tiempo transcurrido en milisegundos, obteniendo el RTT exacto sin necesidad de sincronizar los relojes de las máquinas.
