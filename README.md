# Pong Multijugador Online — Telemática

**Proyecto N1: Internet — Arquitectura y Protocolos**  
**Universidad EAFIT / Curso de Telemática**  
**Arquitectura:** Cliente-Servidor Autoritativo sobre Sockets Berkeley UDP  
**Protocolo de Capa de Aplicación:** `MyAppGameProtocol` (Binario, Network Byte Order / Big-Endian)  

---

## 1. Introducción

El presente proyecto implementa un videojuego multijugador retro en red (*Networked Game*), **Pong**, siguiendo un modelo arquitectónico **Cliente-Servidor Autoritativo**. El desarrollo tiene como objetivo la profundización práctica en la programación de sockets Berkeley de bajo nivel, la gestión de aplicaciones concurrentes en tiempo real, el diseño formal y serialización manual de protocolos de capa de aplicación en formato binario, y la mitigación de los retos inherentes al transporte no orientado a conexión (UDP) como la latencia, desorden y pérdida de datagramas.

En esta solución, el servidor (`PongServer`) opera como la **única fuente de verdad** del sistema: computa la física de la pelota a paso de tiempo discreto, gestiona los tanteadores, valida y aplica las intenciones de movimiento de los jugadores y arbitra simultáneamente múltiples partidas independientes mediante hilos concurrentes. Por su parte, la aplicación cliente (`PongClient`) captura las entradas del usuario, las transmite periódicamente hacia el servidor y renderiza en pantalla las instantáneas de estado autoritativo recibidas, asegurando una experiencia interactiva a 60 FPS desacoplada del subsistema de red.

Para una descripción detallada de la distribución de responsabilidades del código fuente y los criterios de evaluación entre la lógica de redes y la interfaz visual, consultar el documento complementario:
* [Guía de Distribución de Responsabilidades y Código](docs/distribucion-codigo.md).

---

## 2. Desarrollo

### 2.1. Arquitectura del Sistema

La solución se estructura bajo un estricto desacoplamiento por capas y separación de hilos de ejecución para prevenir bloqueos en la interfaz y garantizar un rendimiento óptimo:

```text
┌─────────────────────────────────────────────────────────────┐
│                 PONG CLIENT (PongClient)                    │
│                                                             │
│  ┌────────────────────────┐      ┌───────────────────────┐  │
│  │   Hilo GUI (Macroquad) │      │  Hilo Red UDP Worker  │  │
│  │   - Bucle a 60 FPS     │      │  - Dueño UdpSocket    │  │
│  │   - Input Teclado      │      │  - Codec Binario      │  │
│  │   - Renderizado 2D     │      │  - Telemetría RTT     │  │
│  └───────────┬────────────┘      └───────────▲───────────┘  │
│              │                               │              │
│              │    NetworkBridge (Canal MPSC) │              │
│              └───────────────►◄──────────────┘              │
└──────────────────────────────┼──────────────────────────────┘
                               │ Datagramas UDP
                               ▼ (SOCK_DGRAM)
┌─────────────────────────────────────────────────────────────┐
│                 PONG SERVER (PongServer)                    │
│                                                             │
│  - Socket Berkeley UDP no bloqueante (Puerto configurable)  │
│  - Subsistema de Logging dual (Consola + Archivo de texto)  │
│  - Cola de Emparejamiento (Matchmaking Queue)               │
│  - N Partidas Concurrentes en hilos independientes          │
│  - Motor Físico Autoritativo Puro (Mesa 800x600 virtual)    │
└─────────────────────────────────────────────────────────────┘
```

#### Justificación del Protocolo de Transporte: UDP (`SOCK_DGRAM`)
Para juegos en tiempo real y acción continua como Pong, se seleccionó el protocolo **UDP** (`SOCK_DGRAM`) frente a TCP debido a que:
1. **Baja Latencia y Ausencia de *Head-of-Line Blocking*:** TCP impone retransmisiones obligatorias ante pérdida de segmentos y control de congestión, lo que retiene paquetes posteriores aunque ya contengan datos obsoletos. En Pong, una coordenada de pelota retrasada carece de valor; la física demanda recibir inmediatamente el estado más reciente disponible.
2. **Menor Sobrecarga de Encabezados:** La cabecera UDP consta de tan solo 8 bytes frente a los 20+ bytes de TCP, minimizando el consumo de ancho de banda y la fragmentación a nivel IP.

---

### 2.2. Protocolo de Capa de Aplicación (`MyAppGameProtocol`)

El protocolo está diseñado estrictamente en formato **binario** (sin JSON, XML ni texto plano) y codificado en **Network Byte Order (Big-Endian)**.

#### Formato de la Cabecera Fija (12 Bytes)
Todo datagrama transmitido por la red inicia con la siguiente cabecera de longitud fija:

| Campo | Tipo / Tamaño | Descripción |
| :--- | :---: | :--- |
| `Magic Byte` | `u8` (1 byte) | Identificador constante del protocolo: `0x50` (carácter ASCII `'P'`). |
| `Version` | `u8` (1 byte) | Versión del protocolo de capa de aplicación (actualmente `1`). |
| `Message Type` | `u16` (2 bytes) | Opcode identificador del tipo de mensaje (`1` a `10`) en Big-Endian. |
| `Sequence Number`| `u32` (4 bytes) | Contador secuencial Big-Endian para ordenamiento y descarte de duplicados. |
| `Session ID` | `u16` (2 bytes) | Token de sesión Big-Endian asignado por el servidor (`0` en registro). |
| `Payload Length` | `u16` (2 bytes) | Tamaño en bytes del cuerpo del mensaje en Big-Endian. |

#### Vocabulario de Mensajes (Opcodes)

| Opcode | Nombre | Origen | Propósito / Payload |
| :---: | :--- | :---: | :--- |
| `0x01` | `HELLO_REQ` | Cliente | Solicitud de registro inicial. Contiene longitud y bytes UTF-8 de Nickname y Email. |
| `0x02` | `HELLO_ACK` | Servidor | Confirmación de registro exitoso. Asigna el `Session ID` oficial al cliente. |
| `0x03` | `WAITING_OPPONENT` | Servidor | Notifica al cliente que está en cola de emparejamiento. |
| `0x04` | `MATCH_START` | Servidor | Inicio de partida. Asigna lado de juego (`Left`/`Right`) y nickname del rival. |
| `0x05` | `INPUT_CMD` | Cliente | Intención de movimiento (`MoveUp`, `MoveDown`, `Stop`) del jugador. |
| `0x06` | `STATE_SYNC` | Servidor | Instantánea autoritativa: posición de pelota $(X, Y)$, palas $(Y_1, Y_2)$ y marcadores. |
| `0x07` | `GAME_OVER` | Servidor | Fin de partida por límite de puntos (7 goles) o desconexión del oponente. |
| `0x08` | `PING` | Cliente | Datagrama con token `u32` para medición de RTT. |
| `0x09` | `PONG` | Servidor | Eco inmediato del token `u32` para cálculo de latencia de ida y vuelta. |
| `0x0A` | `LEAVE` | Cliente | Abandono ordenado y explícito de la partida o lobby al presionar ESC / salir. |

Para especificaciones exactas de cada byte, diagramas de bits y reglas de transición, consultar [`docs/protocolo.md`](docs/protocolo.md).

---

### 2.3. Concurrencia y Servidor Autoritativo

El servidor implementa concurrencia multihilo estructurada en Rust:
1. **Hilo Receptor Principal:** Escucha datagramas entrantes mediante un socket Berkeley no bloqueante. Atiende registros iniciales (`HELLO_REQ`), gestiona la cola FIFO de emparejamiento y enruta los comandos entrantes a las partidas activas correspondientes.
2. **Hilos de Partida Independientes ($N$ Partidas Concurrentes):** Al emparejarse dos clientes, se instancia un hilo dedicado (`MatchThread`) con un motor de física independiente (`GameEngine`). Las partidas no comparten bloqueos de memoria entre sí; la caída, latencia o desconexión de una pareja de jugadores jamás degrada la experiencia de las demás partidas concurrentes.
3. **Manejo de Desorden e Idempotencia:** El protocolo utiliza aritmética modular de secuencias para descartar instantáneas o comandos antiguos provocados por fluctuaciones de red. Si un cliente pierde la confirmación `HELLO_ACK` y reenvía su saludo, el servidor responde de manera idempotente sin duplicar al usuario en la cola.

---

### 2.4. Subsistema de Logging Dual (`Logger`)

Cumpliendo con la especificación de `ProyectoPongN1.md`, el servidor cuenta con un subsistema de registro de eventos que imprime por consola estándar y escribe en el archivo de texto indicado al arrancar:
* **Datagramas Entrantes (RX):** Marca de tiempo, IP:Puerto de origen, cantidad de bytes recibidos y tipo de mensaje decodificado.
* **Datagramas Salientes (TX):** Marca de tiempo, IP:Puerto de destino, cantidad de bytes transmitidos y tipo de mensaje.
* **Eventos de Ciclo de Vida:** Conexiones, emparejamientos, inicios de partidas, anotaciones de puntos, victorias y desconexiones por timeout.

---

### 2.5. Compilación y Ejecución

El proyecto incluye un `Makefile` compatible con entornos POSIX (Linux/macOS) y herramientas estándar de Rust.

#### Requisitos Previos
* **Rust:** Versión 1.85+ (el proyecto incluye `rust-toolchain.toml` fijando la versión 1.90.0 recomendada).
* **Make** (opcional, para usar el Makefile).

#### Instrucciones con Makefile
```bash
# Compilar tanto el servidor (headless) como el cliente gráfico
make all

# Compilar únicamente el servidor headless (sin librerías de ventana, para AWS EC2)
make server

# Compilar únicamente el cliente gráfico
make client

# Ejecutar la suite de 41 pruebas automatizadas
make test
```

#### Ejecución del Servidor
Según el estándar requerido en la rúbrica (`./server <PORT> <Log File>`):
```bash
# Ejecutar binario directo:
./server 8080 server.log

# O alternativamente mediante Cargo:
cargo run --bin server --no-default-features -- 8080 server.log
```

#### Ejecución del Cliente
```bash
# 1. Modo interactivo (abre formulario en pantalla para ingresar servidor, nickname y correo):
cargo run --bin client

# 2. Modo directo por línea de comandos:
cargo run --bin client -- 127.0.0.1:8080 Jugador1 jugador1@example.com

# 3. Modo demostración offline (sin conexión de red):
cargo run --bin client -- --demo
```

---

### 2.6. Guía de Despliegue en AWS Academy (EC2)

Para poner en producción el servidor en la nube de Amazon Web Services (AWS Academy):

1. **Lanzar una Instancia EC2:**
   * **AMI:** Ubuntu Server 24.04 LTS o Amazon Linux 2023 (`x86_64`).
   * **Tipo de Instancia:** `t2.micro` (disponible en la capa gratuita / AWS Academy).
2. **Configurar el Grupo de Seguridad (Security Group):**
   * Es mandatorio habilitar una regla de entrada (*Inbound Rule*):
     * **Tipo:** Custom UDP Rule
     * **Protocolo:** UDP
     * **Rango de Puertos:** `8080` (o el puerto elegido para el servidor).
     * **Origen (Source):** `Anywhere-IPv4` (`0.0.0.0/0`).
     * *(Adicionalmente mantener puerto SSH 22 TCP para administración).*
3. **Instalación y Ejecución en el Servidor EC2:**
   ```bash
   # Conectarse vía SSH a la instancia
   ssh -i "tu-llave.pem" ubuntu@<IP_PUBLICA_EC2>

   # Instalar Rust y utilidades de compilación
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source $HOME/.cargo/env
   sudo apt-get update && sudo apt-get install -y build-essential git

   # Clonar el repositorio
   git clone https://github.com/buendiant/Pong0.git
   cd Pong0

   # Compilar el servidor en modo release headless
   make server

   # Ejecutar el servidor en segundo plano con nohup o tmux
   nohup ./server 8080 server_aws.log > /dev/null 2>&1 &
   ```
4. **Conexión de los Jugadores:**
   * Cada jugador ejecuta su cliente gráfico en su computadora local e ingresa en el formulario la `<IP_PUBLICA_EC2>:8080`.

---

## 3. Conclusiones

1. **Eficacia de los Protocolos Binarios en Redes de Tiempo Real:** La implementación manual de `MyAppGameProtocol` evidenció la drástica reducción de sobrecarga frente a formatos textuales como JSON. Un paquete de estado binario de 43 bytes con serialización Big-Endian permitió mantener una cadencia de transmisión fluida sin comprometer el ancho de banda ni exigir fragmentación a nivel IP.
2. **Tolerancia a Fallos sobre UDP:** La naturaleza no confiable de UDP se solventó eficazmente mediante números de secuencia con aritmética modular, retransmisión periódica de intenciones (inputs) y estados autoritativos redundantes, confirmando que la pérdida esporádica de datagramas no degrada la jugabilidad siempre que el servidor mantenga la autoridad física.
3. **Seguridad y Autoridad del Servidor:** El modelo Cliente-Servidor Autoritativo demostró ser la arquitectura adecuada para evitar trampas (*anti-cheat*) y desincronizaciones de estado (*desync*). El cliente se limita a solicitar intenciones de aceleración, mientras que el cómputo determinista de trayectorias, rebotes y puntuaciones recae exclusivamente en el servidor.
4. **Concurrencia Segura en Rust:** La utilización del sistema de tipos de Rust y abstracciones como `Arc`, `Mutex` y canales libres de contención permitió gestionar múltiples partidas en paralelo con garantías completas contra condiciones de carrera en memoria (*data races*).
5. **Medición de RTT Precisa:** El mecanismo de eco con tokens de correlación (`PING`/`PONG`) demostró cómo es posible computar la latencia de red exacta entre dos nodos distribuidos empleando únicamente relojes monotónicos locales, sin depender de complejas sincronizaciones horarias de protocolo NTP.

---

## 4. Referencias

* **Beej's Guide to Network Programming:** Using Internet Sockets. Brian "Beej Jorgensen" Hall. Disponible en: <https://beej.us/guide/bgnet/>
* **Postel, J. (1980):** RFC 768 — *User Datagram Protocol*. Internet Engineering Task Force (IETF).
* **Bernier, Y. W. (2001):** *Latency Compensating Methods in Client/Server In-game Protocol Design and Optimization*. Valve Software.
* **The Rust Programming Language:** *Building a Multithreaded Web Server & Sockets API*. Steve Klabnik & Carol Nichols.
* **Macroquad Graphics Engine:** Minimal Rust game engine documentation. Disponible en: <https://macroquad.rs/>
