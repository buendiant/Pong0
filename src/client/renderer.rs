use macroquad::prelude::*;

use crate::client::types::{GameStateSnapshot, MatchStatus, PlayerSide};

pub const VIRTUAL_WIDTH: f32 = 800.0;
pub const VIRTUAL_HEIGHT: f32 = 600.0;
pub const PADDLE_WIDTH: f32 = 14.0;
pub const PADDLE_HEIGHT: f32 = 90.0;
pub const BALL_SIZE: f32 = 12.0;

/// Dibuja la escena completa adaptándose a la resolución actual de la ventana.
pub fn render_frame(state: &GameStateSnapshot) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Cálculo de escalado manteniendo la relación de aspecto 4:3 (800x600)
    let scale_x = screen_w / VIRTUAL_WIDTH;
    let scale_y = screen_h / VIRTUAL_HEIGHT;
    let scale = scale_x.min(scale_y);

    let offset_x = (screen_w - VIRTUAL_WIDTH * scale) * 0.5;
    let offset_y = (screen_h - VIRTUAL_HEIGHT * scale) * 0.5;

    // Fondo global oscuro
    clear_background(Color::new(0.04, 0.05, 0.07, 1.0));

    // Mesa de juego delimitada
    let to_screen_x = |vx: f32| offset_x + vx * scale;
    let to_screen_y = |vy: f32| offset_y + vy * scale;

    // Fondo del campo de juego
    draw_rectangle(
        to_screen_x(0.0),
        to_screen_y(0.0),
        VIRTUAL_WIDTH * scale,
        VIRTUAL_HEIGHT * scale,
        Color::new(0.07, 0.08, 0.12, 1.0),
    );

    // Bordes superior e inferior con efecto neón
    let border_color = Color::new(0.2, 0.3, 0.45, 1.0);
    draw_line(
        to_screen_x(0.0),
        to_screen_y(2.0),
        to_screen_x(VIRTUAL_WIDTH),
        to_screen_y(2.0),
        3.0 * scale,
        border_color,
    );
    draw_line(
        to_screen_x(0.0),
        to_screen_y(VIRTUAL_HEIGHT - 2.0),
        to_screen_x(VIRTUAL_WIDTH),
        to_screen_y(VIRTUAL_HEIGHT - 2.0),
        3.0 * scale,
        border_color,
    );

    // Red central punteada
    let dash_h = 15.0;
    let dash_gap = 12.0;
    let mut current_y = 10.0;
    while current_y < VIRTUAL_HEIGHT - 10.0 {
        draw_rectangle(
            to_screen_x(VIRTUAL_WIDTH * 0.5 - 2.0),
            to_screen_y(current_y),
            4.0 * scale,
            dash_h * scale,
            Color::new(0.25, 0.3, 0.4, 0.8),
        );
        current_y += dash_h + dash_gap;
    }

    // Palas de los jugadores
    // Pala izquierda (P1 - Neón Cian)
    let p1_x = 22.0;
    let p1_y = state.paddle_left_y - PADDLE_HEIGHT * 0.5;
    let p1_color = Color::new(0.0, 0.85, 1.0, 1.0);
    draw_rectangle(
        to_screen_x(p1_x),
        to_screen_y(p1_y),
        PADDLE_WIDTH * scale,
        PADDLE_HEIGHT * scale,
        p1_color,
    );

    // Pala derecha (P2 - Neón Naranja)
    let p2_x = VIRTUAL_WIDTH - 22.0 - PADDLE_WIDTH;
    let p2_y = state.paddle_right_y - PADDLE_HEIGHT * 0.5;
    let p2_color = Color::new(1.0, 0.55, 0.05, 1.0);
    draw_rectangle(
        to_screen_x(p2_x),
        to_screen_y(p2_y),
        PADDLE_WIDTH * scale,
        PADDLE_HEIGHT * scale,
        p2_color,
    );

    // Pelota con brillo
    let ball_radius = (BALL_SIZE * 0.5) * scale;
    draw_circle(
        to_screen_x(state.ball_x),
        to_screen_y(state.ball_y),
        ball_radius,
        Color::new(1.0, 1.0, 1.0, 1.0),
    );

    // Marcadores principales (HUD superior)
    let score_font_size = (48.0 * scale).max(20.0);
    let p1_score_str = format!("{}", state.score_left);
    let p2_score_str = format!("{}", state.score_right);

    let p1_measure = measure_text(&p1_score_str, None, score_font_size as u16, 1.0);
    let _p2_measure = measure_text(&p2_score_str, None, score_font_size as u16, 1.0);

    draw_text(
        &p1_score_str,
        to_screen_x(VIRTUAL_WIDTH * 0.5 - 80.0) - p1_measure.width,
        to_screen_y(65.0),
        score_font_size,
        p1_color,
    );

    draw_text(
        &p2_score_str,
        to_screen_x(VIRTUAL_WIDTH * 0.5 + 80.0),
        to_screen_y(65.0),
        score_font_size,
        p2_color,
    );

    // Nombres de los jugadores
    let label_font_size = (18.0 * scale).max(12.0);
    let p1_badge = if state.my_side == PlayerSide::Left {
        format!("{} [TÚ]", state.p1_name)
    } else {
        state.p1_name.clone()
    };
    let p2_badge = if state.my_side == PlayerSide::Right {
        format!("{} [TÚ]", state.p2_name)
    } else {
        state.p2_name.clone()
    };

    draw_text(
        &p1_badge,
        to_screen_x(30.0),
        to_screen_y(35.0),
        label_font_size,
        Color::new(0.7, 0.9, 1.0, 0.9),
    );

    let p2_badge_measure = measure_text(&p2_badge, None, label_font_size as u16, 1.0);
    draw_text(
        &p2_badge,
        to_screen_x(VIRTUAL_WIDTH - 30.0) - p2_badge_measure.width,
        to_screen_y(35.0),
        label_font_size,
        Color::new(1.0, 0.8, 0.6, 0.9),
    );

    // Diagnósticos y telemetría (RTT / FPS)
    let diag_font_size = (14.0 * scale).max(10.0);
    let fps_text = format!("FPS: {}", get_fps());
    let ping_text = match state.ping_ms {
        Some(ms) => format!("RTT: {} ms", ms),
        None => "Red: Desconectada (Simulador Local)".to_string(),
    };
    draw_text(
        &format!("{} | {}", fps_text, ping_text),
        to_screen_x(30.0),
        to_screen_y(VIRTUAL_HEIGHT - 15.0),
        diag_font_size,
        Color::new(0.5, 0.55, 0.65, 0.8),
    );

    // Mensaje de controles inferior
    let controls_msg = "[W / S o Flechas: Mover]  |  [ESPACIO: Listo/Reiniciar]  |  [ESC: Salir]";
    let controls_measure = measure_text(controls_msg, None, diag_font_size as u16, 1.0);
    draw_text(
        controls_msg,
        to_screen_x(VIRTUAL_WIDTH - 30.0) - controls_measure.width,
        to_screen_y(VIRTUAL_HEIGHT - 15.0),
        diag_font_size,
        Color::new(0.5, 0.55, 0.65, 0.8),
    );

    // Superposición de estado y eventos centrales
    draw_status_overlay(state, to_screen_x, to_screen_y, scale);
}

/// Dibuja carteles flotantes según el estado actual de la partida.
fn draw_status_overlay<FX, FY>(
    state: &GameStateSnapshot,
    to_x: FX,
    to_y: FY,
    scale: f32,
) where
    FX: Fn(f32) -> f32,
    FY: Fn(f32) -> f32,
{
    let overlay_box = |msg: &str, sub: &str, accent_color: Color| {
        let box_w = 460.0 * scale;
        let box_h = 120.0 * scale;
        let box_x = to_x(VIRTUAL_WIDTH * 0.5) - box_w * 0.5;
        let box_y = to_y(VIRTUAL_HEIGHT * 0.5) - box_h * 0.5;

        // Fondo traslúcido
        draw_rectangle(
            box_x,
            box_y,
            box_w,
            box_h,
            Color::new(0.05, 0.06, 0.1, 0.92),
        );
        // Marco de acento
        draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0 * scale, accent_color);

        let font_title = (28.0 * scale).max(16.0);
        let title_m = measure_text(msg, None, font_title as u16, 1.0);
        draw_text(
            msg,
            to_x(VIRTUAL_WIDTH * 0.5) - title_m.width * 0.5,
            box_y + 45.0 * scale,
            font_title,
            accent_color,
        );

        let font_sub = (16.0 * scale).max(11.0);
        let sub_m = measure_text(sub, None, font_sub as u16, 1.0);
        draw_text(
            sub,
            to_x(VIRTUAL_WIDTH * 0.5) - sub_m.width * 0.5,
            box_y + 85.0 * scale,
            font_sub,
            Color::new(0.8, 0.85, 0.9, 0.9),
        );
    };

    match &state.status {
        MatchStatus::OfflineDemo => {
            // No bloquea la vista, solo indica arriba si se desea
        }
        MatchStatus::Connecting => {
            overlay_box(
                "CONECTANDO AL SERVIDOR",
                "Enviando solicitud HELLO_REQ por socket UDP...",
                Color::new(0.2, 0.8, 1.0, 1.0),
            );
        }
        MatchStatus::WaitingForOpponent => {
            overlay_box(
                "ESPERANDO OPONENTE",
                "Registrado en el lobby. Buscando rival para emparejamiento...",
                Color::new(1.0, 0.8, 0.2, 1.0),
            );
        }
        MatchStatus::InMatch => {}
        MatchStatus::PointScored { scorer, .. } => {
            let (texto, color) = match scorer {
                PlayerSide::Left => ("¡PUNTO PARA P1!", Color::new(0.0, 0.85, 1.0, 1.0)),
                PlayerSide::Right => ("¡PUNTO PARA P2!", Color::new(1.0, 0.55, 0.05, 1.0)),
                PlayerSide::Spectator => ("¡PUNTO ANOTADO!", WHITE),
            };
            overlay_box(texto, "Reanudando saque autoritativo...", color);
        }
        MatchStatus::GameOver { winner } => {
            let (texto, sub, color) = match (winner, state.my_side) {
                (PlayerSide::Left, PlayerSide::Left) | (PlayerSide::Right, PlayerSide::Right) => (
                    "¡VICTORIA!",
                    "Has ganado la partida. Presiona ESPACIO para reiniciar.",
                    Color::new(0.1, 1.0, 0.4, 1.0),
                ),
                (PlayerSide::Spectator, _) => (
                    "FIN DE LA PARTIDA",
                    "Presiona ESPACIO para reiniciar.",
                    WHITE,
                ),
                _ => (
                    "FIN DE LA PARTIDA",
                    "El rival ha alcanzado la puntuación máxima. ESPACIO para reiniciar.",
                    Color::new(1.0, 0.3, 0.3, 1.0),
                ),
            };
            overlay_box(texto, sub, color);
        }
        MatchStatus::Disconnected { reason } => {
            overlay_box(
                "DESCONECTADO",
                &format!("Motivo: {}", reason),
                Color::new(1.0, 0.2, 0.2, 1.0),
            );
        }
    }
}
