//! Pruebas con sockets reales: cuatro clientes, dos partidas y fallos de red.
use pong_lib::protocol::{Action, EndReason, Message, Packet, Side, StateSync};
use pong_lib::server::{PongServer, ServerConfig};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static TEST_ID: AtomicUsize = AtomicUsize::new(0);

struct Host {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<io::Result<()>>>,
    log: PathBuf,
}
impl Host {
    fn start(config: ServerConfig) -> Self {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let log = std::env::temp_dir().join(format!("pong0-{}-{id}.log", std::process::id()));
        let server = PongServer::bind("127.0.0.1:0", &log, config).unwrap();
        let address = server.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = thread::spawn(move || server.run(flag));
        Self {
            address,
            stop,
            handle: Some(handle),
            log,
        }
    }
    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap().unwrap();
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.shutdown();
        let _ = std::fs::remove_file(&self.log);
    }
}

struct Client {
    socket: UdpSocket,
    session: u16,
    sequence: u32,
    nickname: String,
}
impl Client {
    fn register(address: SocketAddr, nickname: &str) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.connect(address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut client = Self {
            socket,
            session: 0,
            sequence: 1,
            nickname: nickname.into(),
        };
        client.hello();
        let reply = client.wait_for(Duration::from_secs(2), |message| {
            matches!(message, Message::HelloAck { .. })
        });
        if let Message::HelloAck {
            assigned_session_id,
        } = reply.message
        {
            assert_eq!(assigned_session_id, reply.session_id);
            assert_ne!(assigned_session_id, 0);
            client.session = assigned_session_id;
        }
        client
    }
    fn hello(&self) {
        self.send_as(
            0,
            0,
            Message::HelloReq {
                nickname: self.nickname.clone(),
                email: format!("{}@example.com", self.nickname),
            },
        );
    }
    fn send_as(&self, session_id: u16, sequence: u32, message: Message) {
        let bytes = Packet {
            session_id,
            sequence,
            message,
        }
        .encode()
        .unwrap();
        assert_eq!(self.socket.send(&bytes).unwrap(), bytes.len());
    }
    fn input(&mut self, action: Action) {
        self.send_as(self.session, self.sequence, Message::InputCmd { action });
        self.sequence = self.sequence.wrapping_add(1);
    }
    fn receive(&self) -> Option<Packet> {
        let mut bytes = [0; 1024];
        match self.socket.recv(&mut bytes) {
            Ok(length) => Some(Packet::decode(&bytes[..length]).unwrap()),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                None
            }
            Err(error) => panic!("recepción UDP: {error}"),
        }
    }
    fn wait_for(&self, timeout: Duration, predicate: impl Fn(&Message) -> bool) -> Packet {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(packet) = self.receive()
                && predicate(&packet.message)
            {
                return packet;
            }
        }
        panic!(
            "{} no recibió el mensaje esperado en {timeout:?}",
            self.nickname
        );
    }
    fn start(&self, side: Side, opponent: &str) {
        let packet = self.wait_for(Duration::from_secs(2), |message| {
            matches!(message, Message::MatchStart { .. })
        });
        assert_eq!(packet.session_id, self.session);
        assert_eq!(
            packet.message,
            Message::MatchStart {
                side,
                opponent_nickname: opponent.into()
            }
        );
    }
    fn state(&self, predicate: impl Fn(&StateSync) -> bool) -> StateSync {
        let packet = self.wait_for(
            Duration::from_secs(2),
            |message| matches!(message, Message::StateSync(state) if predicate(state)),
        );
        assert_eq!(packet.session_id, self.session);
        match packet.message {
            Message::StateSync(state) => state,
            _ => unreachable!(),
        }
    }
}

fn config() -> ServerConfig {
    ServerConfig {
        tick_interval: Duration::from_millis(10),
        session_timeout: Duration::from_secs(3),
        ..Default::default()
    }
}

#[test]
fn four_clients_play_two_independent_matches_and_log_both_directions() {
    let mut host = Host::start(config());
    let mut ana = Client::register(host.address, "Ana");
    let mut bob = Client::register(host.address, "Bob");
    let mut carla = Client::register(host.address, "Carla");
    let mut dan = Client::register(host.address, "Dan");
    ana.start(Side::Left, "Bob");
    bob.start(Side::Right, "Ana");
    carla.start(Side::Left, "Dan");
    dan.start(Side::Right, "Carla");
    assert!(
        ana.session != bob.session && ana.session != carla.session && ana.session != dan.session
    );
    assert!(
        bob.session != carla.session && bob.session != dan.session && carla.session != dan.session
    );

    ana.input(Action::MoveUp);
    bob.input(Action::Stop);
    carla.input(Action::MoveDown);
    dan.input(Action::Stop);
    let first = ana.state(|state| state.paddle_left_y < 280.0 && state.ball_x > 400.0);
    let second = carla.state(|state| state.paddle_left_y > 320.0 && state.ball_x > 400.0);
    assert_eq!(first.paddle_right_y, 300.0);
    assert_eq!(second.paddle_right_y, 300.0);
    bob.state(|state| state.paddle_left_y < 280.0);
    dan.state(|state| state.paddle_left_y > 320.0);
    host.shutdown();
    let log = std::fs::read_to_string(&host.log).unwrap();
    assert_eq!(log.matches("EMPAREJAMIENTO").count(), 2);
    assert!(log.contains("RX origen=") && log.contains("TX destino="));
    assert!(log.contains("tipo=StateSync") && log.contains("tipo=InputCmd"));
}

#[test]
fn hello_retries_keep_the_same_session_and_do_not_create_a_second_match() {
    let mut host = Host::start(config());
    let ana = Client::register(host.address, "Ana");
    ana.hello();
    let repeated = ana.wait_for(Duration::from_secs(1), |message| {
        matches!(message, Message::HelloAck { .. })
    });
    assert_eq!(repeated.session_id, ana.session);
    let bob = Client::register(host.address, "Bob");
    ana.start(Side::Left, "Bob");
    bob.start(Side::Right, "Ana");
    ana.hello();
    let replay = ana.wait_for(Duration::from_secs(1), |message| {
        matches!(message, Message::HelloAck { .. })
    });
    assert_eq!(replay.session_id, ana.session);
    ana.start(Side::Left, "Bob");
    host.shutdown();
    let log = std::fs::read_to_string(&host.log).unwrap();
    assert_eq!(log.matches("REGISTRO").count(), 2);
    assert_eq!(log.matches("EMPAREJAMIENTO").count(), 1);
}

#[test]
fn forged_sessions_stale_inputs_and_server_only_messages_cannot_move_paddles() {
    let mut host = Host::start(config());
    let mut ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.start(Side::Left, "Bob");
    bob.start(Side::Right, "Ana");
    ana.sequence = 10;
    ana.input(Action::Stop);
    ana.send_as(
        ana.session,
        9,
        Message::InputCmd {
            action: Action::MoveUp,
        },
    );
    ana.send_as(
        ana.session,
        10,
        Message::InputCmd {
            action: Action::MoveDown,
        },
    );
    bob.send_as(
        ana.session,
        100,
        Message::InputCmd {
            action: Action::MoveDown,
        },
    );
    bob.send_as(
        bob.session,
        1,
        Message::GameOver {
            winner_side: Side::Right,
            reason: EndReason::ScoreLimit,
        },
    );
    bob.socket.send(&[0x50, 1]).unwrap();
    let deadline = Instant::now() + Duration::from_millis(150);
    while Instant::now() < deadline {
        if let Some(Packet {
            message: Message::StateSync(state),
            ..
        }) = ana.receive()
        {
            assert_eq!(state.paddle_left_y, 300.0);
            assert_eq!(state.paddle_right_y, 300.0);
        }
    }
    // Un paquete falsificado no consumió el número de secuencia de Ana.
    ana.input(Action::MoveUp);
    ana.state(|state| state.paddle_left_y < 290.0);
    host.shutdown();
    let log = std::fs::read_to_string(&host.log).unwrap();
    assert!(log.contains("fuera de orden") && log.contains("origen no corresponde"));
    assert!(log.contains("exclusivo del servidor") && log.contains("RX RECHAZADO"));
}

#[test]
fn missing_stop_packet_does_not_leave_a_paddle_moving_forever() {
    let host = Host::start(ServerConfig {
        input_timeout: Duration::from_millis(80),
        ..config()
    });
    let mut ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.start(Side::Left, "Bob");
    ana.input(Action::MoveUp);
    let deadline = Instant::now() + Duration::from_millis(220);
    while Instant::now() < deadline {
        let _ = ana.receive();
    }
    // Vaciar snapshots anteriores sin esperar que un flujo continuo se agote.
    let stopped = ana.state(|state| state.ball_x > 440.0);
    let later = ana.state(|state| state.ball_x > stopped.ball_x + 10.0);
    assert_eq!(stopped.paddle_left_y, later.paddle_left_y);
    assert!(later.paddle_left_y < 300.0 && later.paddle_left_y > 240.0);
    assert_ne!(ana.session, bob.session);
}

#[test]
fn timeout_ends_only_the_affected_match_and_replays_the_result() {
    let mut host = Host::start(ServerConfig {
        session_timeout: Duration::from_millis(450),
        control_interval: Duration::from_millis(80),
        terminal_retention: Duration::from_millis(300),
        ..config()
    });
    let mut ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    let mut carla = Client::register(host.address, "Carla");
    let mut dan = Client::register(host.address, "Dan");
    ana.start(Side::Left, "Bob");
    carla.start(Side::Left, "Dan");
    let deadline = Instant::now() + Duration::from_millis(650);
    let mut result = None;
    while Instant::now() < deadline {
        ana.input(Action::Stop);
        carla.input(Action::Stop);
        dan.input(Action::Stop);
        if let Some(packet) = ana.receive()
            && matches!(packet.message, Message::GameOver { .. })
        {
            result = Some(packet.message);
            break;
        }
        thread::sleep(Duration::from_millis(8));
    }
    let expected = Message::GameOver {
        winner_side: Side::Left,
        reason: EndReason::OpponentDisconnected,
    };
    assert_eq!(result, Some(expected.clone()));
    // La segunda pareja continúa recibiendo físicas después del fin de la primera.
    carla.input(Action::MoveDown);
    dan.input(Action::Stop);
    carla.state(|state| state.paddle_left_y > 310.0);
    ana.hello();
    assert_eq!(
        ana.wait_for(Duration::from_secs(1), |message| matches!(
            message,
            Message::GameOver { .. }
        ))
        .message,
        expected
    );
    host.shutdown();
    assert_ne!(ana.session, bob.session);
    let log = std::fs::read_to_string(&host.log).unwrap();
    assert!(log.contains("FIN partida=1") && !log.contains("FIN partida=2"));
}

#[test]
fn expired_lobby_sessions_release_capacity_and_are_not_paired() {
    let mut host = Host::start(ServerConfig {
        session_timeout: Duration::from_millis(180),
        input_timeout: Duration::from_millis(80),
        max_sessions: 2,
        ..config()
    });
    let old = Client::register(host.address, "Old");
    thread::sleep(Duration::from_millis(300));
    let ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.start(Side::Left, "Bob");
    bob.start(Side::Right, "Ana");
    assert_ne!(old.session, ana.session);
    host.shutdown();
    let log = std::fs::read_to_string(&host.log).unwrap();
    assert!(log.contains("SESIÓN CERRADA id=1"));
    assert_eq!(log.matches("EMPAREJAMIENTO").count(), 1);
}

#[test]
fn malformed_and_oversized_packets_do_not_take_lobby_slots() {
    let mut host = Host::start(ServerConfig {
        max_sessions: 2,
        ..config()
    });
    let intruder = UdpSocket::bind("127.0.0.1:0").unwrap();
    intruder.connect(host.address).unwrap();
    intruder.send(&vec![0x50; 1024]).unwrap();
    let mut invalid = Packet {
        session_id: 0,
        sequence: 1,
        message: Message::HelloReq {
            nickname: "Bad".into(),
            email: "bad@example.com".into(),
        },
    }
    .encode()
    .unwrap();
    invalid[11] = 255; // Longitud declarada que no coincide con el datagrama.
    intruder.send(&invalid).unwrap();
    let ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.start(Side::Left, "Bob");
    bob.start(Side::Right, "Ana");
    host.shutdown();
    let log = std::fs::read_to_string(&host.log).unwrap();
    assert!(log.contains("demasiado grande") && log.contains("LengthMismatch"));
    assert_eq!(log.matches("REGISTRO").count(), 2);
}

#[test]
fn completed_match_releases_capacity_and_a_new_session_can_use_the_same_endpoint() {
    let mut host = Host::start(ServerConfig {
        max_sessions: 2,
        session_timeout: Duration::from_millis(250),
        input_timeout: Duration::from_millis(80),
        terminal_retention: Duration::from_millis(120),
        control_interval: Duration::from_millis(40),
        ..config()
    });
    let mut ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.start(Side::Left, "Bob");
    ana.wait_for(Duration::from_secs(2), |message| {
        matches!(message, Message::GameOver { .. })
    });
    let previous = ana.session;
    thread::sleep(Duration::from_millis(220));
    ana.hello();
    let ack = ana.wait_for(Duration::from_secs(1), |message| {
        matches!(message, Message::HelloAck { .. })
    });
    ana.session = ack.session_id;
    assert_ne!(previous, ana.session);
    let carla = Client::register(host.address, "Carla");
    ana.start(Side::Left, "Carla");
    carla.start(Side::Right, "Ana");
    host.shutdown();
    assert_ne!(bob.session, ana.session);
    let log = std::fs::read_to_string(&host.log).unwrap();
    assert_eq!(log.matches("EMPAREJAMIENTO").count(), 2);
    assert!(log.contains("SESIÓN CERRADA id=1") && log.contains("SESIÓN CERRADA id=2"));
}
