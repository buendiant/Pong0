use macroquad::prelude::*;
use pong_lib::client::{run_client_gui, spawn_offline_demo_worker, NetworkBridge};

fn window_conf() -> Conf {
    Conf {
        window_title: "Pong Telemática - Cliente Multijugador".to_string(),
        window_width: 800,
        window_height: 600,
        window_resizable: true,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("Iniciando Pong Client...");
    println!("Inicializando puente desacoplado entre interfaz gráfica y red...");

    // 1. Crear el canal bidireccional
    let (bridge, endpoint) = NetworkBridge::new();

    // 2. Por ahora arrancamos el trabajador en modo simulación/demo local.
    // Cuando implementes tu socket UDP de red, simplemente reemplazarás esta llamada
    // por la función que lance tu hilo de red con sockets Berkeley UDP:
    //
    //    mi_modulo_red::spawn_udp_worker(endpoint, "127.0.0.1:8080", "MiNick", "yo@correo.com");
    //
    println!("Iniciando hilo de simulación visual (Modo Demo Offline)...");
    spawn_offline_demo_worker(endpoint);

    // 3. Ejecutar el bucle de la interfaz gráfica a 60 FPS
    run_client_gui(bridge).await;

    println!("Cliente cerrado correctamente.");
}
