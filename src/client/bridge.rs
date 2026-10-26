use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::client::types::{ClientInput, GameStateSnapshot, MatchStatus, PlayerSide};

/// Extremo del puente que reside en el hilo del motor gráfico (GUI).
pub struct NetworkBridge {
    pub input_sender: Sender<ClientInput>,
    pub state_receiver: Receiver<GameStateSnapshot>,
}

/// Extremo del puente que se entrega al hilo de red UDP.
pub struct NetworkWorkerEndpoint {
    pub input_receiver: Receiver<ClientInput>,
    pub state_sender: Sender<GameStateSnapshot>,
}

impl NetworkBridge {
    /// Inicializa los canales MPSC bidireccionales y desacoplados.
    pub fn new() -> (Self, NetworkWorkerEndpoint) {
        let (in_tx, in_rx) = mpsc::channel();
        let (state_tx, state_rx) = mpsc::channel();

        (
            NetworkBridge {
                input_sender: in_tx,
                state_receiver: state_rx,
            },
            NetworkWorkerEndpoint {
                input_receiver: in_rx,
                state_sender: state_tx,
            },
        )
    }

    /// Envía un comando de entrada al hilo de red de manera no bloqueante.
    pub fn send_input(&self, input: ClientInput) {
        let _ = self.input_sender.send(input);
    }

    /// Drena el canal de estados para obtener la instantánea más reciente.
    /// Retorna `true` si hubo alguna actualización en el fotograma actual.
    pub fn poll_latest_state(&self, current: &mut GameStateSnapshot) -> bool {
        let mut updated = false;
        loop {
            match self.state_receiver.try_recv() {
                Ok(snapshot) => {
                    *current = snapshot;
                    updated = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Preservar la razón de desconexión o el resultado ya recibido.
                    if !matches!(
                        current.status,
                        MatchStatus::GameOver { .. } | MatchStatus::Disconnected { .. }
                    ) {
                        current.status = MatchStatus::Disconnected {
                            reason: "Canal de red cerrado".to_string(),
                        };
                        updated = true;
                    }
                    break;
                }
            }
        }
        updated
    }
}

/// Inicia un trabajador de simulación local (modo Offline / Demo).
/// Permite jugar y verificar el renderizador visual inmediatamente sin requerir
/// aún la implementación del socket UDP del servidor.
pub fn spawn_offline_demo_worker(endpoint: NetworkWorkerEndpoint) {
    thread::spawn(move || {
        let mut state = GameStateSnapshot {
            status: MatchStatus::OfflineDemo,
            p1_name: "Tú (Local)".to_string(),
            p2_name: "Bot Demo".to_string(),
            my_side: PlayerSide::Left,
            ..GameStateSnapshot::default()
        };

        let mut ball_speed_x: f32 = 280.0;
        let mut ball_speed_y: f32 = 180.0;
        let paddle_speed: f32 = 360.0;
        let mut p1_dir: f32 = 0.0;

        let tick_duration = Duration::from_millis(16); // ~60 Hz de simulación
        let mut last_instant = Instant::now();

        loop {
            let now = Instant::now();
            let dt = (now - last_instant).as_secs_f32().min(0.05);
            last_instant = now;

            // 1. Procesar todos los inputs recibidos desde el GUI
            while let Ok(input) = endpoint.input_receiver.try_recv() {
                match input {
                    ClientInput::MoveUp => p1_dir = -1.0,
                    ClientInput::MoveDown => p1_dir = 1.0,
                    ClientInput::Stop => p1_dir = 0.0,
                    ClientInput::Ready => {
                        // Reiniciar marcador si terminó
                        if matches!(state.status, MatchStatus::GameOver { .. }) {
                            state.score_left = 0;
                            state.score_right = 0;
                            state.status = MatchStatus::OfflineDemo;
                            state.ball_x = 400.0;
                            state.ball_y = 300.0;
                            ball_speed_x = 280.0;
                            ball_speed_y = 180.0;
                        }
                    }
                    ClientInput::Quit => return,
                }
            }

            // 2. Simulación física elemental local
            if matches!(
                state.status,
                MatchStatus::OfflineDemo | MatchStatus::InMatch
            ) {
                // Movimiento pala izquierda (controlada por el jugador)
                state.paddle_left_y += p1_dir * paddle_speed * dt;
                state.paddle_left_y = state.paddle_left_y.clamp(45.0, 555.0);

                // Movimiento pala derecha (IA simple de seguimiento)
                let target_y = state.ball_y;
                let diff = target_y - state.paddle_right_y;
                let bot_dir = if diff.abs() > 10.0 {
                    diff.signum()
                } else {
                    0.0
                };
                state.paddle_right_y += bot_dir * (paddle_speed * 0.75) * dt;
                state.paddle_right_y = state.paddle_right_y.clamp(45.0, 555.0);

                // Movimiento de la pelota
                state.ball_x += ball_speed_x * dt;
                state.ball_y += ball_speed_y * dt;
                state.ball_vx = ball_speed_x;
                state.ball_vy = ball_speed_y;

                // Colisión con bordes superior e inferior (Y: 10 .. 590)
                if state.ball_y <= 15.0 {
                    state.ball_y = 15.0;
                    ball_speed_y = ball_speed_y.abs();
                } else if state.ball_y >= 585.0 {
                    state.ball_y = 585.0;
                    ball_speed_y = -ball_speed_y.abs();
                }

                // Colisión con pala izquierda (X ~ 30)
                let p1_top = state.paddle_left_y - 45.0;
                let p1_bot = state.paddle_left_y + 45.0;
                if state.ball_x <= 36.0
                    && state.ball_x >= 20.0
                    && state.ball_y >= p1_top
                    && state.ball_y <= p1_bot
                {
                    ball_speed_x = ball_speed_x.abs() * 1.05; // Acelerar levemente
                    let hit_offset = (state.ball_y - state.paddle_left_y) / 45.0;
                    ball_speed_y = hit_offset * 320.0;
                    state.ball_x = 37.0;
                }

                // Colisión con pala derecha (X ~ 770)
                let p2_top = state.paddle_right_y - 45.0;
                let p2_bot = state.paddle_right_y + 45.0;
                if state.ball_x >= 764.0
                    && state.ball_x <= 780.0
                    && state.ball_y >= p2_top
                    && state.ball_y <= p2_bot
                {
                    ball_speed_x = -ball_speed_x.abs() * 1.05;
                    let hit_offset = (state.ball_y - state.paddle_right_y) / 45.0;
                    ball_speed_y = hit_offset * 320.0;
                    state.ball_x = 763.0;
                }

                // Gol en arco izquierdo (Punto para P2)
                if state.ball_x < 0.0 {
                    state.score_right += 1;
                    if state.score_right >= 7 {
                        state.status = MatchStatus::GameOver {
                            winner: PlayerSide::Right,
                        };
                    } else {
                        state.ball_x = 400.0;
                        state.ball_y = 300.0;
                        ball_speed_x = 280.0;
                        ball_speed_y = 150.0;
                    }
                }

                // Gol en arco derecho (Punto para P1)
                if state.ball_x > 800.0 {
                    state.score_left += 1;
                    if state.score_left >= 7 {
                        state.status = MatchStatus::GameOver {
                            winner: PlayerSide::Left,
                        };
                    } else {
                        state.ball_x = 400.0;
                        state.ball_y = 300.0;
                        ball_speed_x = -280.0;
                        ball_speed_y = -150.0;
                    }
                }
            }

            // 3. Emitir instantánea al hilo del GUI
            if endpoint.state_sender.send(state.clone()).is_err() {
                // El receptor del GUI fue destruido (ventana cerrada)
                return;
            }

            thread::sleep(tick_duration);
        }
    });
}
