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
