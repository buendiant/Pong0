#![cfg(feature = "client")]

//! El mismo trabajador UDP de la GUI, probado mediante NetworkBridge sin GPU.
use pong_lib::client::{
    ClientInput, GameStateSnapshot, MatchStatus, NetworkBridge, NetworkConfig, PlayerSide,
    spawn_udp_network_worker,
};
use pong_lib::protocol::{Action, EndReason, Message, Packet, Side, StateStatus, StateSync};
use pong_lib::server::{PongServer, ServerConfig};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static NEXT_HOST: AtomicUsize = AtomicUsize::new(0);

struct Host {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<io::Result<()>>>,
    log: PathBuf,
}

impl Host {
    fn start() -> Self {
        let id = NEXT_HOST.fetch_add(1, Ordering::Relaxed);
        let log = std::env::temp_dir().join(format!(
            "pong0-client-network-{}-{id}.log",
            std::process::id()
        ));
        let config = ServerConfig {
            tick_interval: Duration::from_millis(10),
            ..ServerConfig::default()
        };
        let server = PongServer::bind("127.0.0.1:0", &log, config).unwrap();
        let address = server.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        Self {
            address,
            stop,
            handle: Some(thread::spawn(move || server.run(flag))),
            log,
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let result = handle.join();
            if !thread::panicking() {
                result.unwrap().unwrap();
            }
        }
        let _ = std::fs::remove_file(&self.log);
    }
}

struct Worker {
    bridge: NetworkBridge,
    state: GameStateSnapshot,
    handle: Option<JoinHandle<()>>,
    nickname: String,
}

impl Worker {
    fn connect(address: SocketAddr, nickname: &str) -> Self {
        let (bridge, endpoint) = NetworkBridge::new();
        let config = NetworkConfig::new(
            address.to_string(),
            nickname.into(),
            format!("{nickname}@example.com"),
        )
        .unwrap();
        let handle = spawn_udp_network_worker(endpoint, config).unwrap();
        Self {
            bridge,
            state: GameStateSnapshot::default(),
            handle: Some(handle),
            nickname: nickname.into(),
        }
    }

    fn input(&self, input: ClientInput) {
        self.bridge.send_input(input);
    }

    fn wait_for(
        &mut self,
        timeout: Duration,
        predicate: impl Fn(&GameStateSnapshot) -> bool,
    ) -> GameStateSnapshot {
        let deadline = Instant::now() + timeout;
        loop {
            if predicate(&self.state) {
                return self.state.clone();
            }
            assert!(
                Instant::now() < deadline,
                "{} no recibió el estado esperado en {timeout:?}; último={:?}",
                self.nickname,
                self.state
            );
            match self
                .bridge
                .state_receiver
                .recv_timeout(Duration::from_millis(25))
            {
                Ok(state) => self.state = state,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => panic!("{} perdió el puente: {error}", self.nickname),
            }
        }
    }

    fn observe_for(&mut self, duration: Duration, predicate: impl Fn(&GameStateSnapshot) -> bool) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            if let Ok(state) = self
                .bridge
                .state_receiver
                .recv_timeout(Duration::from_millis(10))
            {
                self.state = state;
            }
            assert!(
                predicate(&self.state),
                "estado inesperado: {:?}",
                self.state
            );
        }
    }

    fn quit(&mut self) {
        self.input(ClientInput::Quit);
        let handle = self.handle.take().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !handle.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(handle.is_finished(), "Quit no terminó el hilo UDP");
        handle.join().unwrap();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.bridge.send_input(ClientInput::Quit);
            let deadline = Instant::now() + Duration::from_secs(2);
            while !handle.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            if handle.is_finished() {
                let result = handle.join();
                if !thread::panicking() {
                    result.unwrap();
                }
            }
        }
    }
}

fn playing(state: &GameStateSnapshot) -> bool {
    matches!(
        state.status,
        MatchStatus::InMatch | MatchStatus::PointScored { .. }
    )
}

#[test]
fn four_workers_keep_inputs_alive_and_sync_names_sides_rtt_and_independent_games() {
    let host = Host::start();
    let mut ana = Worker::connect(host.address, "Ana");
    ana.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
    let mut bob = Worker::connect(host.address, "Bob");
    let a = ana.wait_for(Duration::from_secs(2), playing);
    let b = bob.wait_for(Duration::from_secs(2), playing);
    assert_eq!(a.my_side, PlayerSide::Left);
    assert_eq!(b.my_side, PlayerSide::Right);
    assert_eq!((a.p1_name.as_str(), a.p2_name.as_str()), ("Ana", "Bob"));
    assert_eq!((&b.p1_name, &b.p2_name), (&a.p1_name, &a.p2_name));

    let mut carla = Worker::connect(host.address, "Carla");
    carla.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
    let mut dan = Worker::connect(host.address, "Dan");
    carla.wait_for(Duration::from_secs(2), playing);
    dan.wait_for(Duration::from_secs(2), playing);

    // La GUI envía el cambio una sola vez. El trabajador debe repetirlo durante
    // más de 250 ms, o la pala se detendría antes de alcanzar este umbral.
    ana.input(ClientInput::MoveUp);
    carla.input(ClientInput::MoveDown);
    let first = bob.wait_for(Duration::from_secs(2), |state| state.paddle_left_y < 180.0);
    let second = dan.wait_for(Duration::from_secs(2), |state| state.paddle_left_y > 420.0);
    assert_eq!(first.paddle_right_y, 300.0);
    assert_eq!(second.paddle_right_y, 300.0);
    ana.input(ClientInput::Stop);
    carla.input(ClientInput::Stop);
    let with_ping = ana.wait_for(Duration::from_secs(3), |state| state.ping_ms.is_some());
    assert!(with_ping.ping_ms.unwrap() < 2000);

    // Salir de la primera pareja no interrumpe la segunda simulación.
    ana.quit();
    bob.wait_for(Duration::from_secs(2), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Right,
            }
    });
    carla.input(ClientInput::MoveUp);
    dan.wait_for(Duration::from_secs(2), |state| {
        playing(state) && state.paddle_left_y < 400.0
    });
}

#[test]
fn ready_after_game_over_registers_a_new_session_and_can_play_again() {
    let host = Host::start();
    let mut ana = Worker::connect(host.address, "Ana");
    ana.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
    let mut bob = Worker::connect(host.address, "Bob");
    ana.wait_for(Duration::from_secs(2), playing);
    bob.wait_for(Duration::from_secs(2), playing);
    ana.quit();
    bob.wait_for(Duration::from_secs(2), |state| {
        matches!(state.status, MatchStatus::GameOver { .. })
    });
    bob.input(ClientInput::Ready);
    bob.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
    let mut carla = Worker::connect(host.address, "Carla");
    let rematch = bob.wait_for(Duration::from_secs(2), playing);
    carla.wait_for(Duration::from_secs(2), playing);
    assert_eq!(rematch.my_side, PlayerSide::Left);
    assert_eq!(rematch.p1_name, "Bob");
    assert_eq!(rematch.p2_name, "Carla");
    assert_eq!((rematch.score_left, rematch.score_right), (0, 0));
    bob.input(ClientInput::MoveDown);
    carla.wait_for(Duration::from_secs(2), |state| state.paddle_left_y > 330.0);
}

#[test]
fn a_complete_authoritative_match_reaches_seven_and_both_workers_keep_the_result() {
    let host = Host::start();
    let mut ana = Worker::connect(host.address, "Ana");
    ana.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
    let mut bob = Worker::connect(host.address, "Bob");
    ana.wait_for(Duration::from_secs(2), playing);
    bob.wait_for(Duration::from_secs(2), playing);

    // Una partida normal por goles, sin modificar el motor, la física ni el
    // límite de siete puntos. Ambas palas quedan arriba para dejar pasar.
    ana.input(ClientInput::MoveUp);
    bob.input(ClientInput::MoveUp);
    ana.wait_for(Duration::from_secs(3), |state| {
        state.paddle_left_y <= 50.0 && state.paddle_right_y <= 50.0
    });
    ana.input(ClientInput::Stop);
    bob.input(ClientInput::Stop);
    let first = ana.wait_for(Duration::from_secs(45), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
            && state.score_left == 7
    });
    let second = bob.wait_for(Duration::from_secs(2), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
            && state.score_left == 7
    });
    assert_eq!((first.score_left, first.score_right), (7, 0));
    assert_eq!(
        (first.score_left, first.score_right),
        (second.score_left, second.score_right)
    );
    let final_state = |state: &GameStateSnapshot| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
            && (state.score_left, state.score_right) == (7, 0)
    };
    ana.observe_for(Duration::from_millis(100), final_state);
    bob.observe_for(Duration::from_millis(100), final_state);
}

fn fake_server() -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    socket
}

fn receive_until(
    socket: &UdpSocket,
    timeout: Duration,
    predicate: impl Fn(&Message) -> bool,
) -> (Packet, SocketAddr) {
    let deadline = Instant::now() + timeout;
    let mut buffer = [0; 1024];
    while Instant::now() < deadline {
        match socket.recv_from(&mut buffer) {
            Ok((length, address)) => {
                let packet = Packet::decode(&buffer[..length]).unwrap();
                if predicate(&packet.message) {
                    return (packet, address);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => panic!("servidor de prueba: {error}"),
        }
    }
    panic!("el servidor de prueba no recibió el mensaje esperado en {timeout:?}");
}

fn reply(
    socket: &UdpSocket,
    address: SocketAddr,
    sequence: u32,
    session_id: u16,
    message: Message,
) {
    let bytes = Packet {
        sequence,
        session_id,
        message,
    }
    .encode()
    .unwrap();
    socket.send_to(&bytes, address).unwrap();
}

fn accept_fake(socket: &UdpSocket, address: SocketAddr, session: u16) {
    reply(
        socket,
        address,
        1,
        session,
        Message::HelloAck {
            assigned_session_id: session,
        },
    );
    reply(socket, address, 2, session, Message::WaitingOpponent);
}

fn snapshot(score: u8) -> StateSync {
    StateSync {
        ball_x: 450.0,
        ball_y: 300.0,
        paddle_left_y: 250.0,
        paddle_right_y: 350.0,
        score_left: score,
        score_right: 1,
        status: StateStatus::InGame,
    }
}

#[test]
fn lost_registration_is_retried_and_reordered_control_cannot_reset_a_match() {
    let fake = fake_server();
    let mut worker = Worker::connect(fake.local_addr().unwrap(), "Ana");
    let (_, address) = receive_until(&fake, Duration::from_secs(2), |message| {
        matches!(message, Message::HelloReq { .. })
    });
    // Se pierde el primer HELLO completo; debe llegar otro desde el mismo socket.
    let (_, retry_address) = receive_until(&fake, Duration::from_secs(2), |message| {
        matches!(message, Message::HelloReq { .. })
    });
    assert_eq!(address, retry_address);
    accept_fake(&fake, address, 77);
    reply(
        &fake,
        address,
        3,
        77,
        Message::MatchStart {
            side: Side::Left,
            opponent_nickname: "Rival".into(),
        },
    );
    reply(&fake, address, 5, 77, Message::StateSync(snapshot(2)));
    worker.wait_for(Duration::from_secs(2), |state| state.score_left == 2);

    reply(&fake, address, 4, 77, Message::StateSync(snapshot(0)));
    reply(
        &fake,
        address,
        6,
        77,
        Message::MatchStart {
            side: Side::Left,
            opponent_nickname: "Rival".into(),
        },
    );
    reply(&fake, address, 7, 77, Message::WaitingOpponent);
    reply(&fake, address, 8, 999, Message::StateSync(snapshot(0)));
    // Un mensaje en la dirección equivocada no debe consumir la secuencia de
    // estados válidos del servidor.
    reply(
        &fake,
        address,
        100,
        77,
        Message::InputCmd {
            action: Action::MoveUp,
        },
    );
    worker.observe_for(Duration::from_millis(150), |state| {
        state.score_left == 2 && playing(state)
    });
    reply(&fake, address, 9, 77, Message::StateSync(snapshot(3)));
    let valid = worker.wait_for(Duration::from_secs(2), |state| state.score_left == 3);
    assert_eq!(valid.my_side, PlayerSide::Left);
    assert_eq!(
        (valid.p1_name.as_str(), valid.p2_name.as_str()),
        ("Ana", "Rival")
    );
    assert!(playing(&valid));

    reply(
        &fake,
        address,
        10,
        77,
        Message::GameOver {
            winner_side: Side::Left,
            reason: EndReason::ScoreLimit,
        },
    );
    worker.wait_for(Duration::from_secs(2), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
    });
    reply(&fake, address, 11, 77, Message::StateSync(snapshot(0)));
    reply(
        &fake,
        address,
        12,
        77,
        Message::MatchStart {
            side: Side::Left,
            opponent_nickname: "Rival".into(),
        },
    );
    worker.observe_for(Duration::from_millis(200), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
            && state.score_left == 3
    });
}

#[test]
fn rtt_is_measured_only_from_a_matching_pong_token() {
    let fake = fake_server();
    let mut worker = Worker::connect(fake.local_addr().unwrap(), "Ana");
    let (_, address) = receive_until(&fake, Duration::from_secs(2), |message| {
        matches!(message, Message::HelloReq { .. })
    });
    accept_fake(&fake, address, 12);
    worker.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
    let (ping, _) = receive_until(&fake, Duration::from_secs(2), |message| {
        matches!(message, Message::Ping { .. })
    });
    let token = match ping.message {
        Message::Ping { token } => token,
        _ => unreachable!(),
    };
    reply(
        &fake,
        address,
        3,
        12,
        Message::Pong {
            token: token.wrapping_add(1),
        },
    );
    worker.observe_for(Duration::from_millis(40), |state| state.ping_ms.is_none());
    reply(&fake, address, 4, 12, Message::Pong { token });
    let measured = worker.wait_for(Duration::from_secs(1), |state| state.ping_ms.is_some());
    assert!(measured.ping_ms.unwrap() >= 30);
}

#[test]
fn game_over_before_match_start_keeps_the_result_and_recovers_final_metadata() {
    let fake = fake_server();
    let mut worker = Worker::connect(fake.local_addr().unwrap(), "Ana");
    let (_, address) = receive_until(&fake, Duration::from_secs(2), |message| {
        matches!(message, Message::HelloReq { .. })
    });
    accept_fake(&fake, address, 81);
    reply(
        &fake,
        address,
        3,
        81,
        Message::GameOver {
            winner_side: Side::Left,
            reason: EndReason::ScoreLimit,
        },
    );
    worker.wait_for(Duration::from_secs(2), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
    });
    reply(
        &fake,
        address,
        4,
        81,
        Message::MatchStart {
            side: Side::Right,
            opponent_nickname: "Rival".into(),
        },
    );
    let mut finished = snapshot(7);
    finished.score_right = 2;
    finished.status = StateStatus::Finished;
    reply(&fake, address, 5, 81, Message::StateSync(finished));
    let final_state = worker.wait_for(Duration::from_secs(2), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
            && state.my_side == PlayerSide::Right
            && (state.score_left, state.score_right) == (7, 2)
    });
    assert_eq!(
        (final_state.p1_name.as_str(), final_state.p2_name.as_str()),
        ("Rival", "Ana")
    );
    reply(&fake, address, 6, 81, Message::StateSync(snapshot(0)));
    reply(
        &fake,
        address,
        7,
        81,
        Message::MatchStart {
            side: Side::Right,
            opponent_nickname: "Rival".into(),
        },
    );
    worker.observe_for(Duration::from_millis(150), |state| {
        state.status
            == MatchStatus::GameOver {
                winner: PlayerSide::Left,
            }
            && state.my_side == PlayerSide::Right
            && (state.score_left, state.score_right) == (7, 2)
    });
}

#[test]
fn silent_registered_server_times_out_and_ready_restarts_registration() {
    let fake = fake_server();
    let mut worker = Worker::connect(fake.local_addr().unwrap(), "Ana");
    let (_, original_address) = receive_until(&fake, Duration::from_secs(2), |message| {
        matches!(message, Message::HelloReq { .. })
    });
    accept_fake(&fake, original_address, 55);
    worker.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
    worker.wait_for(Duration::from_secs(5), |state| {
        matches!(state.status, MatchStatus::Disconnected { .. })
    });
    worker.input(ClientInput::Ready);
    let deadline = Instant::now() + Duration::from_secs(2);
    let new_address = loop {
        let (packet, address) = receive_until(
            &fake,
            deadline.saturating_duration_since(Instant::now()),
            |message| matches!(message, Message::HelloReq { .. }),
        );
        if address != original_address {
            assert_eq!(packet.session_id, 0);
            break address;
        }
    };
    accept_fake(&fake, new_address, 56);
    worker.wait_for(Duration::from_secs(2), |state| {
        state.status == MatchStatus::WaitingForOpponent
    });
}

#[test]
fn closing_the_state_channel_preserves_the_last_game_over_in_the_bridge() {
    let (bridge, endpoint) = NetworkBridge::new();
    endpoint
        .state_sender
        .send(GameStateSnapshot {
            status: MatchStatus::GameOver {
                winner: PlayerSide::Right,
            },
            score_right: 7,
            ..GameStateSnapshot::default()
        })
        .unwrap();
    drop(endpoint);
    let mut state = GameStateSnapshot::default();
    bridge.poll_latest_state(&mut state);
    assert_eq!(
        state.status,
        MatchStatus::GameOver {
            winner: PlayerSide::Right
        }
    );
    assert_eq!(state.score_right, 7);
    bridge.poll_latest_state(&mut state);
    assert_eq!(
        state.status,
        MatchStatus::GameOver {
            winner: PlayerSide::Right
        }
    );
}
