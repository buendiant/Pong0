# Especificación Formal de MyAppGameProtocol

**Versión del Protocolo:** 1.0  
**Capa de Red:** Aplicación (sobre UDP / `SOCK_DGRAM`)  
**Endianness:** Network Byte Order (Big-Endian)  
**Proyecto:** Pong Multijugador Online (Internet: Arquitectura y Protocolos)

---

## 1. Visión General y Objetivos

`MyAppGameProtocol` es un protocolo binario de capa de aplicación diseñado para operar sobre datagramas UDP. Su objetivo es proporcionar comunicación de baja latencia entre la aplicación cliente (`PongClient`) y el servidor autoritativo (`PongServer`).

### Principios de Diseño:
1. **Compacto y Determinista:** Todo mensaje consta de una cabecera fija de 12 bytes seguida de un cuerpo (*payload*) estructurado. No se utiliza texto plano, JSON ni delimitadores de fin de línea.
2. **Independiente de la Arquitectura:** Todos los enteros multibyte (`u16`, `u32`) y flotantes (`f32`) viajan en orden Big-Endian (Network Byte Order).
3. **Tolerancia a Pérdida y Desorden:** Cada datagrama incluye un número de secuencia ascendente (`Sequence Number`) y un identificador de sesión (`Session ID`).

---

## 2. Anatomía de la Cabecera Fija (Header)

Todos los datagramas intercambiados entre cliente y servidor inician obligatoriamente con los siguientes **12 bytes**:

```text
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Magic (0x50) |  Proto Ver    |          Message Type         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Session ID           |         Payload Length        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Descripción de Campos de la Cabecera:

| Offset (Bytes) | Campo | Tipo Rust | Tamaño | Descripción |
| :---: | :--- | :---: | :---: | :--- |
| `0` | **Magic Byte** | `u8` | 1 Byte | Valor constante `0x50` (ASCII `'P'`). Identifica que el paquete pertenece al juego Pong. |
| `1` | **Protocol Version**| `u8` | 1 Byte | Versión actual: `0x01`. |
| `2..4` | **Message Type** | `u16` | 2 Bytes | Identificador del tipo de mensaje (*OpCode*). Big-Endian. |
| `4..8` | **Sequence Number** | `u32` | 4 Bytes | Contador incremental de datagramas para detectar pérdida y desorden. |
| `8..10` | **Session ID** | `u16` | 2 Bytes | ID asignado por el servidor al jugador (`0` antes de ser asignado). |
| `10..12` | **Payload Length** | `u16` | 2 Bytes | Tamaño exacto en bytes del cuerpo que sigue a la cabecera. |

---

## 3. Catálogo de Mensajes (OpCodes)

| OpCode | Nombre | Dirección | Descripción |
| :---: | :--- | :---: | :--- |
| `0x0001` | `HELLO_REQ` | Cliente $\rightarrow$ Servidor | Petición de registro de cliente (Nickname y Email). |
| `0x0002` | `HELLO_ACK` | Servidor $\rightarrow$ Cliente | Confirmación de registro con Session ID asignado. |
| `0x0003` | `WAITING_OPPONENT`| Servidor $\rightarrow$ Cliente | Notificación de espera en lobby de emparejamiento. |
| `0x0004` | `MATCH_START` | Servidor $\rightarrow$ Cliente | Inicio de partida: lado asignado (Left/Right) y rival. |
| `0x0005` | `INPUT_CMD` | Cliente $\rightarrow$ Servidor | Envío de intención de movimiento de pala. |
| `0x0006` | `STATE_SYNC` | Servidor $\rightarrow$ Cliente | Estado autoritativo continuo (físicas, palas y marcador). |
| `0x0007` | `GAME_OVER` | Servidor $\rightarrow$ Cliente | Fin de partida con indicación de ganador y causa. |

---

## 4. Especificación Detallada de Payloads

### 4.1. `0x0001` — `HELLO_REQ` (Registro de Jugador)
Enviado por el cliente al conectarse por primera vez. Utiliza codificación *Length-Prefixed* para cadenas dinámicas.

```text
+---------------+-------------------+---------------+-------------------+
| Nick Len (1B) | Nickname (UTF-8)  | Mail Len (1B) | Email (UTF-8)     |
+---------------+-------------------+---------------+-------------------+
```
* **Campos:**
  * `nick_len` (`u8`, 1 byte): Longitud $L_1$ del nickname en bytes.
  * `nickname` (`[u8; L1]`): Cadena de texto en UTF-8.
  * `mail_len` (`u8`, 1 byte): Longitud $L_2$ del correo en bytes.
  * `email` (`[u8; L2]`): Cadena de texto en UTF-8.
* **Tamaño Total del Payload:** $1 + L_1 + 1 + L_2$ bytes.

---

### 4.2. `0x0002` — `HELLO_ACK` (Confirmación de Registro)
Enviado por el servidor tras aceptar al cliente.

* **Campos:**
  * `assigned_session_id` (`u16`, 2 bytes): ID único de sesión otorgado al cliente.
* **Tamaño Total del Payload:** 2 bytes.

---

### 4.2.1. `0x0003` — `WAITING_OPPONENT` (Espera en Lobby)

No contiene payload: `Payload Length = 0`; el datagrama completo ocupa 12 bytes.
La cabecera identifica la sesión asignada. Recibir este mensaje repetido no crea
una segunda sesión ni modifica una partida ya iniciada.

---

### 4.3. `0x0004` — `MATCH_START` (Inicio de Partida)
Notifica a ambos jugadores que fueron emparejados.

* **Campos:**
  * `side` (`u8`, 1 byte): `0x01` = Pala Izquierda (P1), `0x02` = Pala Derecha (P2).
  * `opponent_nick_len` (`u8`, 1 byte): Longitud $L$ del nickname rival.
  * `opponent_nickname` (`[u8; L]`): Nickname del oponente.
* **Tamaño Total del Payload:** $2 + L$ bytes.

---

### 4.4. `0x0005` — `INPUT_CMD` (Comando de Pala)
Enviado continuamente por el cliente cuando presiona o suelta teclas.

* **Campos:**
  * `action` (`u8`, 1 byte):
    * `0x00`: `Stop` (Pala quieta).
    * `0x01`: `MoveUp` (Mover pala hacia arriba).
    * `0x02`: `MoveDown` (Mover pala hacia abajo).
* **Tamaño Total del Payload:** 1 byte.
* **Datagrama Completo:** 13 bytes (12 de cabecera + 1 de acción).

---

### 4.5. `0x0006` — `STATE_SYNC` (Foto Autoritativa del Servidor)
Enviado por el servidor a ~60 Hz a ambos clientes de la partida.

| Offset en Payload | Campo | Tipo Rust | Tamaño | Descripción |
| :---: | :--- | :---: | :---: | :--- |
| `0..4` | `ball_x` | `f32` (BE) | 4 Bytes | Posición horizontal de la pelota ($0.0 .. 800.0$). |
| `4..8` | `ball_y` | `f32` (BE) | 4 Bytes | Posición vertical de la pelota ($0.0 .. 600.0$). |
| `8..12` | `paddle_left_y` | `f32` (BE) | 4 Bytes | Posición vertical centro pala izquierda ($0.0 .. 600.0$). |
| `12..16` | `paddle_right_y` | `f32` (BE) | 4 Bytes | Posición vertical centro pala derecha ($0.0 .. 600.0$). |
| `16` | `score_left` | `u8` | 1 Byte | Marcador Jugador 1. |
| `17` | `score_right` | `u8` | 1 Byte | Marcador Jugador 2. |
| `18` | `status` | `u8` | 1 Byte | `0` = En Juego, `1` = Gol Izq, `2` = Gol Der, `3` = Fin. |

* **Tamaño Total del Payload:** 19 bytes.
* **Datagrama Completo:** 31 bytes (12 de cabecera + 19 de payload).

---

### 4.6. `0x0007` — `GAME_OVER` (Fin de Partida)
* **Campos:**
  * `winner_side` (`u8`, 1 byte): `0x01` = Ganó P1, `0x02` = Ganó P2.
  * `reason` (`u8`, 1 byte): `0x00` = Puntos máximos alcanzados, `0x01` = Desconexión del oponente.
* **Tamaño Total del Payload:** 2 bytes.

---

## 5. Reglas de procedimiento implementadas en las fases 4 y 5

### Registro y emparejamiento

1. El cliente envía `HELLO_REQ` con Session ID `0`, nickname y email.
2. El servidor asigna un ID no nulo, devuelve `HELLO_ACK` (el ID del payload
   coincide con el de la cabecera) y `WAITING_OPPONENT`.
3. La cola FIFO empareja dos sesiones: la primera recibe lado izquierdo y la
   segunda lado derecho. Se inicia una simulación autoritativa independiente.
4. Cada jugador recibe `MATCH_START` con el nombre de su rival, seguido por
   `STATE_SYNC` a una cadencia objetivo de 60 Hz, dependiente del host.

El máximo predeterminado es de 256 sesiones, incluidas las que conservan un
resultado final. Si se alcanza el límite, se descartan nuevos registros; v1 no
define un mensaje de rechazo. Nunca se crea una sesión desde `INPUT_CMD`.

Un `HELLO_REQ` repetido desde la misma IP:puerto y con el mismo perfil devuelve
la sesión existente y su estado de control (`WAITING_OPPONENT`, `MATCH_START`
o `GAME_OVER`). Puede conservar Session ID `0` o usar el ID ya asignado. No
vuelve a emparejar al jugador ni reinicia sus físicas. Cambiar de perfil en el
mismo endpoint antes de expirar la sesión se rechaza. Un endpoint nuevo debe
registrarse con ID `0`.

### Entradas, secuencias y pérdida de paquetes

- Las entradas contienen intenciones (`Stop`, `MoveUp`, `MoveDown`), nunca
  coordenadas ni marcadores. Solo el servidor mueve palas, pelota y puntuaciones.
- Para aceptar un `INPUT_CMD`, el ID debe pertenecer a la IP:puerto de origen.
  Se comprueba su secuencia contra la última entrada aceptada de esa sesión.
  Duplicados y entradas antiguas se descartan. La comparación modular admite
  `u32::MAX -> 0`; diferencias de medio espacio (`2^31`) o más no son nuevas.
- La primera secuencia de entrada es libre. Los reintentos de registro no
  consumen esta secuencia. El servidor usa un contador de salida global `u32`;
  las secuencias recibidas por un cliente pueden tener saltos.
- Durante una partida, el cliente debe repetir **su intención actual**, incluido
  `Stop`, al menos cada 100 ms con nueva secuencia. Si transcurren 250 ms sin una
  entrada fresca, el servidor detiene la pala. Así, perder una liberación de tecla
  no deja un movimiento permanente.
- Antes de recibir `HELLO_ACK`, el cliente puede repetir `HELLO_REQ` cada 500 ms.
  En lobby debe repetirlo al menos una vez por segundo para conservar la sesión.
  Un registro repetido en partida sirve de señal de vida, pero no mantiene una
  intención de movimiento que ya expiró.
- `MATCH_START` se repite cada 500 ms durante la partida para recuperar la
  pérdida de ese mensaje. El cliente debe tratarlo de forma idempotente.
- Cada `STATE_SYNC` reemplaza el estado anterior; no se retransmiten fotos
  antiguas. El receptor debe descartar estados atrasados por secuencia. Ninguna
  de estas reglas convierte UDP en un transporte de entrega garantizada.

### Fin de partida y limpieza

Una partida termina cuando alguien alcanza **7 puntos**, o cuando un jugador
acumula **10 segundos sin entradas válidas ni registro repetido**. En el segundo
caso pierde el primer jugador cuyo vencimiento detecta el tick (izquierda si
ambos vencen en el mismo tick). Las sesiones de lobby sin actividad también
expiran a los 10 segundos.

Tras un gol, la pelota vuelve al centro y el estado señala el anotador durante
800 ms antes del siguiente saque. Las palas permanecen dentro de los centros
Y `[45, 555]`; la pelota rebota en paredes y palas. La simulación usa subpasos de
hasta 8 ms y limita la velocidad horizontal a 600 px/s tras los rebotes.

El hilo termina al finalizar la partida y el receptor central lo une. Conserva
`GAME_OVER` durante **5 segundos**, lo repite cada 500 ms y lo devuelve ante
consultas válidas de la sesión. Luego elimina ambas sesiones y libera capacidad.
Un nuevo registro con ID `0` después de esa retención inicia otro ciclo. Las
otras partidas continúan durante todo este proceso.

v1 no incorpora mensajes `Ready`, `Quit` ni ping/pong: esos comandos visuales
necesitan una decisión de integración en fase 6. Dejar de enviar actividad se
detecta por timeout; la medición RTT tampoco está implementada en esta fase.

## 6. Validación de datagramas

El codec manual comprueba cabecera mínima, magic, versión, opcode conocido,
longitud exacta, discriminantes válidos y consumo completo del payload. Las
cadenas deben contener entre 1 y 255 **bytes UTF-8**, y el servidor además
rechaza perfiles vacíos tras recortar espacios o con caracteres de control.
Las coordenadas deben ser finitas y pertenecer al campo de juego; `HELLO_ACK`
no puede asignar ID `0`.

El datagrama máximo válido ocupa **524 bytes** (cabecera más dos cadenas de
255 bytes y sus longitudes). El receptor usa un buffer capaz de recibir UDP
completo y descarta los datagramas mayores, para que un paquete truncado no
pueda hacerse pasar por uno válido. Los mensajes exclusivos del servidor que
llegan de un cliente se rechazan. RX, TX, rechazos y eventos se registran en
consola y archivo con acceso sincronizado entre hilos.
