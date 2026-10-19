use pong_lib::server::{PongServer, ServerConfig};
use std::env;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Modo de uso: server <PUERTO> <Archivo Log>");
        return ExitCode::FAILURE;
    }
    let port = match args[1].parse::<u16>() {
        Ok(port) if port != 0 => port,
        _ => {
            eprintln!("Error: el puerto debe estar entre 1 y 65535");
            return ExitCode::FAILURE;
        }
    };
    match PongServer::bind(("0.0.0.0", port), &args[2], ServerConfig::default())
        .and_then(|server| server.run(Arc::new(AtomicBool::new(false))))
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error del servidor: {error}");
            ExitCode::FAILURE
        }
    }
}
