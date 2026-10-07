use std::env;
use std::net::UdpSocket;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Modo de uso: server <PUERTO> <Archivo Log>");
        return;
    }

    let direccion = format!("0.0.0.0{}", args[1]);
    let socket = std::net::UdpSocket::bind(&direccion).expect("No se pudo enlazar al socket");

    println!("Escuchando en {}", direccion);

    let mut buf = [0u8; 1024];

    let (bytes_recibidos, origen) = socket
        .recv_from(&mut buf)
        .expect("Fallo al recibir el paquete");
}
