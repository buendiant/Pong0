use macroquad::prelude::*;
use pong_lib::client::connection::{CLIENT_USAGE, ClientMode, connection_form, parse_client_args};
use pong_lib::client::{
    NetworkBridge, run_client_gui, spawn_offline_demo_worker, spawn_udp_network_worker,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

fn window_conf() -> Conf {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_client_args(&args) {
        Ok(ClientMode::Help) => {
            println!("{CLIENT_USAGE}");
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("{error}\n\n{CLIENT_USAGE}");
            std::process::exit(2);
        }
        _ => {}
    }
    Conf {
        window_title: "Pong Telemática - Cliente Online".to_string(),
        window_width: 800,
        window_height: 600,
        window_resizable: true,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = match parse_client_args(&args) {
        Ok(mode) => mode,
        Err(_) => return,
    };
    let mut form_error = None;

    loop {
        let config = match mode {
            ClientMode::Form => match connection_form(form_error.take()).await {
                Some(config) => config,
                None => return,
            },
            ClientMode::Online(config) => config,
            ClientMode::Demo => {
                let (bridge, endpoint) = NetworkBridge::new();
                spawn_offline_demo_worker(endpoint);
                run_client_gui(bridge).await;
                return;
            }
            ClientMode::Help => return,
        };
        let (bridge, endpoint) = NetworkBridge::new();
        let handle = match spawn_udp_network_worker(endpoint, config) {
            Ok(handle) => handle,
            Err(error) => {
                form_error = Some(format!("No se pudo iniciar la conexión: {error}"));
                mode = ClientMode::Form;
                continue;
            }
        };
        // La GUI solo intercambia mensajes con el puente. El worker posee el socket.
        run_client_gui(bridge).await;
        finish_worker(handle).await;
        return;
    }
}

async fn finish_worker(handle: JoinHandle<()>) {
    // Quit se envía desde run_client_gui; al retornar también se cierra el canal.
    // Esperar sin bloquear el renderizado, incluso si el DNS del SO tarda en volver.
    let deadline = Instant::now() + Duration::from_secs(1);
    while !handle.is_finished() && Instant::now() < deadline {
        clear_background(Color::from_rgba(10, 13, 21, 255));
        let text = "Cerrando conexión...";
        let size = 24.0;
        let measure = measure_text(text, None, size as u16, 1.0);
        draw_text(
            text,
            (screen_width() - measure.width) * 0.5,
            screen_height() * 0.5,
            size,
            WHITE,
        );
        next_frame().await;
    }
    if handle.is_finished() && handle.join().is_err() {
        eprintln!("El trabajador de red terminó inesperadamente");
    }
}
