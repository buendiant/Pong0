/// Tipos de datos compartidos entre la interfaz gráfica (GUI) y la capa de red del cliente.

/// Comandos de entrada generados por el jugador desde el GUI hacia la capa de red.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientInput {
    /// La pala debe moverse hacia arriba
    MoveUp,
    /// La pala debe moverse hacia abajo
    MoveDown,
    /// La pala se detiene (tecla soltada)
    Stop,
    /// Señal de jugador listo / inicio / reconexión (barra espaciadora)
    Ready,
    /// Solicitud de cierre de ventana / desconexión voluntaria
    Quit,
}

/// Lado asignado al jugador dentro de la mesa de Pong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSide {
    /// Jugador 1 (Pala izquierda)
    Left,
    /// Jugador 2 (Pala derecha)
    Right,
    /// Observador neutral
    Spectator,
}

/// Estado actual del ciclo de la partida.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchStatus {
    /// Modo local o demostración sin servidor conectado
    OfflineDemo,
    /// Intentando establecer conexión UDP con el servidor
    Connecting,
    /// Conectado al servidor, esperando a que se empareje un oponente
    WaitingForOpponent,
    /// Partida activa en disputa
    InMatch,
    /// Un jugador acaba de anotar un tanto
    PointScored {
        scorer: PlayerSide,
        reset_timer: f32,
    },
    /// Partida concluida por límite de puntos
    GameOver {
        winner: PlayerSide,
    },
    /// Conexión interrumpida o cerrada por el servidor
    Disconnected {
        reason: String,
    },
}

/// Snapshot completo del estado del juego recibido desde el servidor o simulador.
/// Todas las coordenadas están referenciadas al espacio virtual de 800x600 px.
#[derive(Debug, Clone)]
pub struct GameStateSnapshot {
    /// Posición X de la pelota (0.0 .. 800.0)
    pub ball_x: f32,
    /// Posición Y de la pelota (0.0 .. 600.0)
    pub ball_y: f32,
    /// Velocidad horizontal de la pelota (para interpolación o efectos)
    pub ball_vx: f32,
    /// Velocidad vertical de la pelota
    pub ball_vy: f32,
    /// Posición Y del centro de la pala izquierda (0.0 .. 600.0)
    pub paddle_left_y: f32,
    /// Posición Y del centro de la pala derecha (0.0 .. 600.0)
    pub paddle_right_y: f32,
    /// Marcador del jugador izquierdo (P1)
    pub score_left: u16,
    /// Marcador del jugador derecho (P2)
    pub score_right: u16,
    /// Estado del ciclo de vida de la partida
    pub status: MatchStatus,
    /// Lado que controla este cliente
    pub my_side: PlayerSide,
    /// Latencia de ida y vuelta (RTT / Ping) estimada en milisegundos
    pub ping_ms: Option<u32>,
    /// Nombre visible del Jugador 1
    pub p1_name: String,
    /// Nombre visible del Jugador 2
    pub p2_name: String,
}

impl Default for GameStateSnapshot {
    fn default() -> Self {
        Self {
            ball_x: 400.0,
            ball_y: 300.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            paddle_left_y: 300.0,
            paddle_right_y: 300.0,
            score_left: 0,
            score_right: 0,
            status: MatchStatus::OfflineDemo,
            my_side: PlayerSide::Left,
            ping_ms: None,
            p1_name: "Jugador 1".to_string(),
            p2_name: "Jugador 2".to_string(),
        }
    }
}
