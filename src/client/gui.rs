use macroquad::prelude::*;

use crate::client::bridge::NetworkBridge;
use crate::client::renderer;
use crate::client::types::{ClientInput, GameStateSnapshot};

/// Ejecuta el bucle principal de dibujado y captura de eventos a 60 FPS.
pub async fn run_client_gui(bridge: NetworkBridge) {
    let mut state = GameStateSnapshot::default();
    let mut last_movement: Option<ClientInput> = None;
    prevent_quit();

    loop {
        // 1. Actualizar el estado visual con la última instantánea disponible del puente
        bridge.poll_latest_state(&mut state);

        // 2. Procesar entrada de teclado
        let move_up = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
        let move_down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);

        let current_movement = if move_up && !move_down {
            Some(ClientInput::MoveUp)
        } else if move_down && !move_up {
            Some(ClientInput::MoveDown)
        } else {
            None
        };

        // Solo notificar al puente si el estado de movimiento cambió
        if current_movement != last_movement {
            match current_movement {
                Some(ref cmd) => bridge.send_input(cmd.clone()),
                None => bridge.send_input(ClientInput::Stop),
            }
            last_movement = current_movement;
        }

        // Acciones discretas
        if is_key_pressed(KeyCode::Space) {
            bridge.send_input(ClientInput::Ready);
            last_movement = None;
        }

        if is_key_pressed(KeyCode::Escape) || is_quit_requested() {
            bridge.send_input(ClientInput::Quit);
            break;
        }

        // 3. Renderizar escena completa
        renderer::render_frame(&state);

        // 4. Ceder control al runtime gráfico de Macroquad
        next_frame().await;
    }
}
