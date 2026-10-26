//! Extensiones v1 de fase 6: RTT correlacionado y abandono explícito.
use pong_lib::protocol::{Action, EndReason, Message, Packet, Side, StateStatus};
use pong_lib::server::{PongServer, ServerConfig};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static LOG_ID: AtomicUsize = AtomicUsize::new(0);

struct Host {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<io::Result<()>>>,
    log: PathBuf,
}
impl Host {
    fn new(config: ServerConfig) -> Self {
        let log = std::env::temp_dir().join(format!(
            "pong-ext-{}-{}.log",
            std::process::id(),
            LOG_ID.fetch_add(1, Ordering::Relaxed)
        ));
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
            handle.join().unwrap().unwrap();
        }
        let _ = std::fs::remove_file(&self.log);
    }
}

struct Client {
    socket: UdpSocket,
    session: u16,
}
impl Client {
    fn register(address: SocketAddr, name: &str) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.connect(address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(10)))
            .unwrap();
        let mut client = Self { socket, session: 0 };
        client.send(
            0,
            0,
            Message::HelloReq {
                nickname: name.into(),
                email: format!("{name}@example.com"),
            },
        );
        client.session = client
            .wait(Duration::from_secs(1), |message| {
                matches!(message, Message::HelloAck { .. })
            })
            .session_id;
        assert_ne!(client.session, 0);
        client
    }
    fn send(&self, id: u16, sequence: u32, message: Message) {
        self.socket
            .send(
                &Packet {
                    sequence,
                    session_id: id,
                    message,
                }
                .encode()
                .unwrap(),
            )
            .unwrap();
    }
    fn receive(&self) -> Option<Packet> {
        let mut buffer = [0; 1024];
        match self.socket.recv(&mut buffer) {
            Ok(length) => Some(Packet::decode(&buffer[..length]).unwrap()),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                None
            }
            Err(error) => panic!("recepción: {error}"),
        }
    }
    fn wait(&self, timeout: Duration, predicate: impl Fn(&Message) -> bool) -> Packet {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(packet) = self.receive()
                && predicate(&packet.message)
            {
                return packet;
            }
        }
        panic!("no se recibió el mensaje esperado en {timeout:?}");
    }
}

#[test]
fn extension_layouts_are_exact_big_endian_and_reject_extra_or_missing_bytes() {
    for (message, opcode, payload) in [
        (
            Message::Ping { token: 0x12345678 },
            8,
            vec![0x12, 0x34, 0x56, 0x78],
        ),
        (
            Message::Pong { token: 0x12345678 },
            9,
            vec![0x12, 0x34, 0x56, 0x78],
        ),
        (Message::Leave, 10, vec![]),
    ] {
        let packet = Packet {
            sequence: 0x01020304,
            session_id: 0x0506,
            message,
        };
        let mut expected = vec![0x50, 1, 0, opcode, 1, 2, 3, 4, 5, 6, 0, payload.len() as u8];
        expected.extend_from_slice(&payload);
        assert_eq!(packet.encode().unwrap(), expected);
        assert_eq!(Packet::decode(&expected).unwrap(), packet);
        for length in 0..expected.len() {
            assert!(Packet::decode(&expected[..length]).is_err());
        }
        expected.push(0);
        assert!(Packet::decode(&expected).is_err());
        expected[11] += 1;
        assert!(Packet::decode(&expected).is_err());
    }
}

#[test]
fn ping_echo_requires_owned_session_and_a_fresh_shared_sequence() {
    let host = Host::new(ServerConfig::default());
    let client = Client::register(host.address, "Ana");
    let stranger = UdpSocket::bind("127.0.0.1:0").unwrap();
    stranger.connect(host.address).unwrap();
    stranger
        .send(
            &Packet {
                sequence: 999,
                session_id: client.session,
                message: Message::Ping { token: 999 },
            }
            .encode()
            .unwrap(),
        )
        .unwrap();
    client.send(client.session, 10, Message::Ping { token: 42 });
    let pong = client.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::Pong { token: 42 })
    });
    assert_eq!(pong.session_id, client.session);
    client.send(client.session, 9, Message::Ping { token: 41 });
    client.send(client.session, 10, Message::Ping { token: 40 });
    let deadline = Instant::now() + Duration::from_millis(80);
    while Instant::now() < deadline {
        assert!(!matches!(
            client.receive().map(|p| p.message),
            Some(Message::Pong {
                token: 40 | 41 | 999
            })
        ));
    }
    client.send(client.session, 11, Message::Ping { token: 43 });
    client.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::Pong { token: 43 })
    });
}

#[test]
fn leave_releases_lobby_capacity_without_waiting_for_timeout() {
    let host = Host::new(ServerConfig {
        max_sessions: 1,
        ..Default::default()
    });
    let client = Client::register(host.address, "Ana");
    client.send(client.session, 1, Message::Leave);
    thread::sleep(Duration::from_millis(30));
    let next = Client::register(host.address, "Bob");
    assert_ne!(client.session, next.session);
}

#[test]
fn stale_or_forged_leave_cannot_end_a_match_but_valid_leave_notifies_the_opponent() {
    let host = Host::new(ServerConfig::default());
    let ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::MatchStart { .. })
    });
    bob.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::MatchStart { .. })
    });
    ana.send(ana.session, 20, Message::Ping { token: 20 });
    ana.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::Pong { token: 20 })
    });
    ana.send(ana.session, 19, Message::Leave);
    bob.send(ana.session, 999, Message::Leave);
    ana.send(
        ana.session,
        21,
        Message::InputCmd {
            action: Action::MoveUp,
        },
    );
    ana.wait(
        Duration::from_secs(1),
        |message| matches!(message, Message::StateSync(state) if state.paddle_left_y < 295.0),
    );
    let sent = Instant::now();
    ana.send(ana.session, 22, Message::Leave);
    let result = bob.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::GameOver { .. })
    });
    assert_eq!(
        result.message,
        Message::GameOver {
            winner_side: Side::Right,
            reason: EndReason::OpponentDisconnected
        }
    );
    assert!(sent.elapsed() < Duration::from_millis(500));
}

#[test]
fn ping_heartbeats_keep_sessions_alive_without_extending_paddle_movement() {
    let host = Host::new(ServerConfig {
        tick_interval: Duration::from_millis(10),
        input_timeout: Duration::from_millis(60),
        session_timeout: Duration::from_millis(180),
        ..Default::default()
    });
    let ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::MatchStart { .. })
    });
    ana.send(
        ana.session,
        1,
        Message::InputCmd {
            action: Action::MoveUp,
        },
    );
    let mut sequence = 2;
    let mut last_y = None;
    let deadline = Instant::now() + Duration::from_millis(400);
    let started = Instant::now();
    while Instant::now() < deadline {
        ana.send(ana.session, sequence, Message::Ping { token: sequence });
        bob.send(bob.session, sequence, Message::Ping { token: sequence });
        sequence += 1;
        let until = Instant::now() + Duration::from_millis(25);
        while Instant::now() < until {
            if let Some(packet) = ana.receive() {
                match packet.message {
                    Message::GameOver { .. } => panic!("Ping válido no mantuvo viva la partida"),
                    Message::StateSync(state) if started.elapsed() > Duration::from_millis(200) => {
                        assert_eq!(state.status, StateStatus::InGame);
                        assert!(state.paddle_left_y < 300.0 && state.paddle_left_y > 260.0);
                        if let Some(y) = last_y {
                            assert_eq!(state.paddle_left_y, y);
                        }
                        last_y = Some(state.paddle_left_y);
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(last_y.is_some());
}

#[test]
fn hello_after_a_match_recovers_side_opponent_and_terminal_result() {
    let host = Host::new(ServerConfig::default());
    let ana = Client::register(host.address, "Ana");
    let bob = Client::register(host.address, "Bob");
    ana.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::MatchStart { .. })
    });
    bob.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::MatchStart { .. })
    });
    ana.send(ana.session, 1, Message::Leave);
    bob.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::GameOver { .. })
    });
    // Permitir al receptor central recoger el resultado y unir el hilo terminado.
    thread::sleep(Duration::from_millis(25));
    bob.send(
        0,
        0,
        Message::HelloReq {
            nickname: "Bob".into(),
            email: "Bob@example.com".into(),
        },
    );
    let start = bob.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::MatchStart { .. })
    });
    assert_eq!(start.session_id, bob.session);
    assert_eq!(
        start.message,
        Message::MatchStart {
            side: Side::Right,
            opponent_nickname: "Ana".into()
        }
    );
    let end = bob.wait(Duration::from_secs(1), |message| {
        matches!(message, Message::GameOver { .. })
    });
    assert_eq!(end.session_id, bob.session);
    assert_eq!(
        end.message,
        Message::GameOver {
            winner_side: Side::Right,
            reason: EndReason::OpponentDisconnected
        }
    );
}
