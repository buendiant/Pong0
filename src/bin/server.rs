use std::env;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::UdpSocket;
use std::time::{SystemTime, UNIX_EPOCH};

fn registrar_log(archivo: &mut File, mensaje: &str) {
    let ahora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    println!("{} {}", ahora, mensaje);
    if let Err(e) = writeln!(archivo, "{} {}", ahora, mensaje) {
        eprintln!("Error escribiendo en log: {}", e);
    }
}

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
            eprintln!(
                "Error: No se pudo enlazar al socket en {}: {}",
                direccion, e
            );
            return;
        }
    };

    let mut log_file: File = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(archivo_log)
    {
        Ok(file) => file,
        Err(e) => {
            eprintln!(
                "Error: No se pudo abrir el archivo de log en {}: {}",
                archivo_log, e
            );
            return;
        }
    };

    registrar_log(
        &mut log_file,
        &format!("Servidor Pong iniciado en {}", direccion),
    );
    registrar_log(
        &mut log_file,
        &format!("Archivo de log configurado: {}", archivo_log),
    );
    registrar_log(&mut log_file, "Esperando datagramas UDP...");

    loop {
        let mut buf = [0u8; 1024];

        match socket.recv_from(&mut buf) {
            Ok((bytes_recibidos, origen)) => {
                let evento = format!(
                    "Recibidos {} bytes desde {}: {:?}",
                    bytes_recibidos,
                    origen,
                    &buf[..bytes_recibidos]
                );
                registrar_log(&mut log_file, &evento);
            }
            Err(e) => {
                eprintln!("Fallo al recibir el paquete: {}", e);
                registrar_log(
                    &mut log_file,
                    &format!("Fallo al recibir el paquete: {}", e),
                );
            }
        }
    }
}
