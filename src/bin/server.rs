use std::env;
use std::net::UdpSocket;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Modo de uso: server <PUERTO> <Archivo Log>");
        return;
    }

    let puerto = &args[1];
    let archivo_log = &args[2];
    let direccion = format!("0.0.0.0:{}", puerto);

    let socket = match UdpSocket::bind(&direccion) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: No se pudo enlazar al socket en {}: {}", direccion, e);
            return;
        }
    };

    println!("Servidor Pong iniciado en {}", direccion);
    println!("Archivo de log configurado: {}", archivo_log);
    println!("Esperando datagramas UDP...");

    let mut buf = [0u8; 1024];

    match socket.recv_from(&mut buf) {
        Ok((bytes_recibidos, origen)) => {
            println!(
                "Recibidos {} bytes desde {}: {:?}",
                bytes_recibidos,
                origen,
                &buf[..bytes_recibidos]
            );
        }
        Err(e) => {
            eprintln!("Fallo al recibir el paquete: {}", e);
        }
    }
}
