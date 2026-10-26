//! Trabajador UDP: el socket y el protocolo viven fuera del hilo gráfico.
use super::bridge::NetworkWorkerEndpoint;
use super::types::{ClientInput, GameStateSnapshot, MatchStatus, PlayerSide};
use crate::protocol::{
    Action, MAX_DATAGRAM_LEN, Message, Packet, Side, StateStatus, StateSync, sequence_is_newer,
};
use std::collections::VecDeque;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::TryRecvError;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const HELLO_INTERVAL: Duration = Duration::from_millis(500);
const INPUT_INTERVAL: Duration = Duration::from_millis(50);
const PING_INTERVAL: Duration = Duration::from_secs(1);
const SERVER_TIMEOUT: Duration = Duration::from_secs(3);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(4);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkConfig {
    pub server_addr: String,
    pub nickname: String,
    pub email: String,
}

impl NetworkConfig {
    /// Valida sin resolver DNS ni abrir sockets: se puede llamar desde la GUI.
    pub fn new(server_addr: String, nickname: String, email: String) -> Result<Self, String> {
        let server_addr = server_addr.trim().to_owned();
        let nickname = nickname.trim().to_owned();
        let email = email.trim().to_owned();
        let valid_address = match server_addr.parse::<SocketAddr>() {
            Ok(address) => address.port() != 0,
            Err(_) => server_addr.rsplit_once(':').is_some_and(|(host, port)| {
                !host.is_empty()
                    && !host
                        .chars()
                        .any(|c| c.is_whitespace() || c.is_control() || "[]:/\\".contains(c))
                    && port.parse::<u16>().is_ok_and(|value| value != 0)
            }),
        };
        if !valid_address {
            return Err("Indica un servidor con puerto válido, por ejemplo 127.0.0.1:8080".into());
        }
        for (label, value) in [("nickname", &nickname), ("correo", &email)] {
            if value.is_empty() || value.len() > 255 || value.chars().any(char::is_control) {
                return Err(format!(
                    "El {label} debe tener entre 1 y 255 bytes UTF-8, sin caracteres de control"
                ));
            }
        }
        if email.chars().any(char::is_whitespace)
            || !email.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty() && !domain.is_empty() && !domain.contains('@')
            })
        {
            return Err("Indica un correo válido, por ejemplo jugador@example.com".into());
        }
        Ok(Self {
            server_addr,
            nickname,
            email,
        })
    }
}

pub fn spawn_udp_network_worker(
    endpoint: NetworkWorkerEndpoint,
    config: NetworkConfig,
) -> io::Result<JoinHandle<()>> {
    let validated = NetworkConfig::new(config.server_addr, config.nickname, config.email)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    thread::Builder::new()
        .name("pong-client-network".into())
        .spawn(move || {
            Worker::new(endpoint, validated).run();
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Registering,
    Waiting,
    Playing,
    Finished,
    Disconnected,
}

struct Worker {
    endpoint: NetworkWorkerEndpoint,
    config: NetworkConfig,
    socket: Option<UdpSocket>,
    view: GameStateSnapshot,
    phase: Phase,
    session: u16,
    sequence: u32,
    // Independientes por tipo: un STATE anterior no descarta un ACK retrasado.
    received_sequences: [Option<u32>; 11],
    metadata: Option<(Side, String)>,
    pending_state: Option<StateSync>,
    last_state: Option<(StateSync, Instant)>,
    point_started: Option<Instant>,
    last_received: Instant,
    connect_started: Instant,
    next_hello: Instant,
    next_input: Instant,
    next_ping: Instant,
    ping_token: u32,
    pings: VecDeque<(u32, Instant)>,
    action: Action,
}

impl Worker {
    fn new(endpoint: NetworkWorkerEndpoint, config: NetworkConfig) -> Self {
        let now = Instant::now();
        Self {
            endpoint,
            view: Self::initial_view(&config),
            config,
            socket: None,
            phase: Phase::Registering,
            session: 0,
            sequence: 1,
            received_sequences: [None; 11],
            metadata: None,
            pending_state: None,
            last_state: None,
            point_started: None,
            last_received: now,
            connect_started: now,
            next_hello: now,
            next_input: now,
            next_ping: now,
            ping_token: 1,
            pings: VecDeque::new(),
            action: Action::Stop,
        }
    }

    fn initial_view(config: &NetworkConfig) -> GameStateSnapshot {
        GameStateSnapshot {
            status: MatchStatus::Connecting,
            my_side: PlayerSide::Spectator,
            p1_name: config.nickname.clone(),
            p2_name: "Esperando rival".into(),
            ..GameStateSnapshot::default()
        }
    }

    fn emit(&self) -> bool {
        self.endpoint.state_sender.send(self.view.clone()).is_ok()
    }

    fn open_connection(&mut self) {
        self.leave();
        self.socket = None;
        self.view = Self::initial_view(&self.config);
        self.phase = Phase::Registering;
        self.session = 0;
        self.sequence = 1;
        self.received_sequences = [None; 11];
        self.metadata = None;
        self.pending_state = None;
        self.last_state = None;
        self.point_started = None;
        self.pings.clear();
        let now = Instant::now();
        self.last_received = now;
        self.connect_started = now;
        self.next_hello = now;
        self.next_input = now;
        self.next_ping = now;
        if !self.emit() {
            return;
        }
        // DNS y apertura ocurren exclusivamente en este hilo secundario.
        let result = (|| -> io::Result<UdpSocket> {
            let addresses: Vec<_> = self.config.server_addr.to_socket_addrs()?.collect();
            let address = addresses
                .iter()
                .find(|address| address.is_ipv4())
                .or(addresses.first())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::AddrNotAvailable,
                        "servidor sin direcciones IP",
                    )
                })?;
            let bind = if address.is_ipv4() {
                "0.0.0.0:0"
            } else {
                "[::]:0"
            };
            let socket = UdpSocket::bind(bind)?;
            // Connect en UDP filtra IP:puerto de origen; no crea una conexión TCP.
            socket.connect(address)?;
            socket.set_nonblocking(true)?;
            Ok(socket)
        })();
        match result {
            Ok(socket) => {
                self.socket = Some(socket);
                self.connect_started = Instant::now();
            }
            Err(error) => self.disconnect(format!("No se pudo abrir la conexión: {error}")),
        }
    }

    fn run(mut self) {
        self.open_connection();
        let mut buffer = [0u8; 65_535];
        let mut next_timer_update = Instant::now();
        loop {
            let mut changed = false;
            for _ in 0..64 {
                match self.endpoint.input_receiver.try_recv() {
                    Ok(ClientInput::Quit) | Err(TryRecvError::Disconnected) => {
                        self.leave();
                        return;
                    }
                    Ok(ClientInput::Ready) => {
                        if matches!(self.phase, Phase::Finished | Phase::Disconnected) {
                            self.open_connection();
                        }
                    }
                    Ok(command) => {
                        self.action = match command {
                            ClientInput::MoveUp => Action::MoveUp,
                            ClientInput::MoveDown => Action::MoveDown,
                            _ => Action::Stop,
                        };
                        self.next_input = Instant::now();
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            for _ in 0..64 {
                let Some(socket) = self.socket.as_ref() else {
                    break;
                };
                match socket.recv(&mut buffer) {
                    Ok(length) if length <= MAX_DATAGRAM_LEN => {
                        if let Ok(packet) = Packet::decode(&buffer[..length])
                            && self.accept(packet, Instant::now())
                        {
                            changed = true;
                        }
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        if self.phase != Phase::Finished {
                            self.disconnect(format!("Se interrumpió la comunicación: {error}"));
                        }
                        break;
                    }
                }
            }
            if let Err(error) = self.tick(Instant::now()) {
                self.disconnect(format!("No se pudo enviar al servidor: {error}"));
            }
            if matches!(self.view.status, MatchStatus::PointScored { .. })
                && Instant::now() >= next_timer_update
            {
                self.update_point_timer(Instant::now());
                next_timer_update = Instant::now() + Duration::from_millis(16);
                changed = true;
            }
            if changed && !self.emit() {
                self.leave();
                return;
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn disconnect(&mut self, reason: String) {
        self.phase = Phase::Disconnected;
        self.socket = None;
        self.view.status = MatchStatus::Disconnected { reason };
        self.view.ping_ms = None;
        self.view.ball_vx = 0.0;
        self.view.ball_vy = 0.0;
        let _ = self.emit();
    }

    fn send(&mut self, message: Message) -> io::Result<()> {
        let Some(socket) = self.socket.as_ref() else {
            return Ok(());
        };
        let packet = Packet {
            sequence: self.sequence,
            session_id: self.session,
            message,
        };
        let bytes = packet
            .encode()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.sequence = self.sequence.wrapping_add(1);
        match socket.send(&bytes) {
            Ok(length) if length == bytes.len() => Ok(()),
            // UDP no ofrece entrega; el siguiente temporizador vuelve a enviar.
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(()),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "datagrama incompleto",
            )),
            Err(error) => Err(error),
        }
    }

    fn leave(&mut self) {
        if self.session != 0 && self.socket.is_some() {
            for _ in 0..3 {
                let _ = self.send(Message::Leave);
            }
        }
    }

    fn tick(&mut self, now: Instant) -> io::Result<()> {
        if self.phase == Phase::Finished {
            // Recuperar el lado/nombre si GAME_OVER adelantó al primer MATCH_START.
            if self.metadata.is_none() && now >= self.next_hello {
                self.send(Message::HelloReq {
                    nickname: self.config.nickname.clone(),
                    email: self.config.email.clone(),
                })?;
                self.next_hello = now + HELLO_INTERVAL;
            }
            return Ok(());
        }
        if self.phase == Phase::Disconnected {
            return Ok(());
        }
        if self.session == 0 && now.duration_since(self.connect_started) >= CONNECT_TIMEOUT {
            self.disconnect("El servidor no respondió al registro. ESPACIO para reintentar".into());
            return Ok(());
        }
        if self.session != 0 && now.duration_since(self.last_received) >= SERVER_TIMEOUT {
            self.disconnect("El servidor dejó de responder. ESPACIO para reconectar".into());
            return Ok(());
        }
        if now >= self.next_hello
            && (self.session == 0 || self.phase == Phase::Waiting || self.metadata.is_none())
        {
            self.send(Message::HelloReq {
                nickname: self.config.nickname.clone(),
                email: self.config.email.clone(),
            })?;
            self.next_hello = now + HELLO_INTERVAL;
        }
        if self.session != 0 && now >= self.next_ping {
            let token = self.ping_token;
            self.ping_token = self.ping_token.wrapping_add(1);
            self.send(Message::Ping { token })?;
            self.pings.push_back((token, now));
            while self.pings.len() > 4 {
                self.pings.pop_front();
            }
            self.next_ping = now + PING_INTERVAL;
        }
        if self.phase == Phase::Playing && now >= self.next_input {
            self.send(Message::InputCmd {
                action: self.action,
            })?;
            self.next_input = now + INPUT_INTERVAL;
        }
        Ok(())
    }

    fn accept(&mut self, packet: Packet, now: Instant) -> bool {
        let index = packet.message.message_type() as usize;
        if self.received_sequences[index]
            .is_some_and(|previous| !sequence_is_newer(packet.sequence, previous))
        {
            return false;
        }
        if let Message::HelloAck {
            assigned_session_id,
        } = packet.message
        {
            if assigned_session_id == 0
                || packet.session_id != assigned_session_id
                || (self.session != 0 && self.session != assigned_session_id)
                || matches!(self.phase, Phase::Finished | Phase::Disconnected)
            {
                return false;
            }
            self.session = assigned_session_id;
            if self.phase == Phase::Registering {
                self.phase = Phase::Waiting;
                self.view.status = MatchStatus::WaitingForOpponent;
            }
        } else {
            if self.session == 0
                || packet.session_id != self.session
                || self.phase == Phase::Disconnected
            {
                return false;
            }
            match packet.message {
                Message::WaitingOpponent => {
                    if matches!(self.phase, Phase::Playing | Phase::Finished) {
                        return false;
                    }
                    self.view.status = MatchStatus::WaitingForOpponent;
                }
                Message::MatchStart {
                    side,
                    opponent_nickname,
                } => {
                    if let Some((previous_side, previous_name)) = &self.metadata {
                        if *previous_side != side || *previous_name != opponent_nickname {
                            return false;
                        }
                    } else {
                        self.view.my_side = player_side(side);
                        let (left, right) = if side == Side::Left {
                            (self.config.nickname.clone(), opponent_nickname.clone())
                        } else {
                            (opponent_nickname.clone(), self.config.nickname.clone())
                        };
                        self.view.p1_name = left;
                        self.view.p2_name = right;
                        self.metadata = Some((side, opponent_nickname));
                        if self.phase != Phase::Finished {
                            self.phase = Phase::Playing;
                            self.view.status = MatchStatus::InMatch;
                            self.next_input = now;
                        }
                        if let Some(state) = self.pending_state.take()
                            && (self.phase != Phase::Finished
                                || state.status == StateStatus::Finished)
                        {
                            self.apply_state(state, now);
                        }
                    }
                }
                Message::StateSync(state) => {
                    if self.phase == Phase::Finished {
                        // UDP puede entregar el resultado antes de la foto final.
                        // Completar marcador sin permitir que una foto InGame reabra la partida.
                        let winner = if state.score_left > state.score_right {
                            PlayerSide::Left
                        } else {
                            PlayerSide::Right
                        };
                        if state.status != StateStatus::Finished
                            || state.score_left == state.score_right
                            || self.view.status != (MatchStatus::GameOver { winner })
                        {
                            return false;
                        }
                        self.apply_state(state, now);
                    } else if self.metadata.is_some() {
                        self.apply_state(state, now);
                    } else {
                        self.pending_state = Some(state);
                    }
                }
                Message::GameOver { winner_side, .. } => {
                    if self.phase == Phase::Finished {
                        return false;
                    }
                    // Si falta MATCH_START, la última foto sigue siendo autoritativa.
                    // Copiarla antes de fijar el resultado preserva el marcador en un
                    // abandono, que no tiene una foto Finished posterior del servidor.
                    if let Some(state) = self.pending_state.take() {
                        self.apply_state(state, now);
                    }
                    self.phase = Phase::Finished;
                    self.view.status = MatchStatus::GameOver {
                        winner: player_side(winner_side),
                    };
                    self.view.ball_vx = 0.0;
                    self.view.ball_vy = 0.0;
                }
                Message::Pong { token } => {
                    if self.phase == Phase::Finished {
                        return false;
                    }
                    let Some(position) =
                        self.pings.iter().position(|(pending, _)| *pending == token)
                    else {
                        return false;
                    };
                    let (_, sent) = self
                        .pings
                        .remove(position)
                        .expect("posición de ping válida");
                    if now.duration_since(sent) > SERVER_TIMEOUT {
                        return false;
                    }
                    self.view.ping_ms = Some(
                        now.duration_since(sent)
                            .as_millis()
                            .min(u128::from(u32::MAX)) as u32,
                    );
                }
                _ => return false,
            }
        }
        self.received_sequences[index] = Some(packet.sequence);
        self.last_received = now;
        true
    }

    fn apply_state(&mut self, state: StateSync, now: Instant) {
        let previous = self.last_state.as_ref();
        self.view.ball_vx = 0.0;
        self.view.ball_vy = 0.0;
        if let Some((old, received)) = previous
            && old.status == StateStatus::InGame
            && state.status == StateStatus::InGame
            && (old.score_left, old.score_right) == (state.score_left, state.score_right)
        {
            let dt = now.duration_since(*received).as_secs_f32();
            if (0.003..=0.25).contains(&dt) {
                self.view.ball_vx = ((state.ball_x - old.ball_x) / dt).clamp(-600.0, 600.0);
                self.view.ball_vy = ((state.ball_y - old.ball_y) / dt).clamp(-320.0, 320.0);
            }
        }
        let same_point = previous.is_some_and(|(old, _)| {
            old.status == state.status
                && old.score_left == state.score_left
                && old.score_right == state.score_right
        });
        self.view.ball_x = state.ball_x;
        self.view.ball_y = state.ball_y;
        self.view.paddle_left_y = state.paddle_left_y;
        self.view.paddle_right_y = state.paddle_right_y;
        self.view.score_left = u16::from(state.score_left);
        self.view.score_right = u16::from(state.score_right);
        self.view.status = match state.status {
            StateStatus::InGame => {
                self.point_started = None;
                MatchStatus::InMatch
            }
            StateStatus::LeftPoint | StateStatus::RightPoint => {
                if !same_point {
                    self.point_started = Some(now);
                }
                MatchStatus::PointScored {
                    scorer: if state.status == StateStatus::LeftPoint {
                        PlayerSide::Left
                    } else {
                        PlayerSide::Right
                    },
                    reset_timer: 0.8,
                }
            }
            StateStatus::Finished => {
                self.phase = Phase::Finished;
                let winner = if state.score_left > state.score_right {
                    PlayerSide::Left
                } else {
                    PlayerSide::Right
                };
                MatchStatus::GameOver { winner }
            }
        };
        self.last_state = Some((state, now));
        self.update_point_timer(now);
    }

    fn update_point_timer(&mut self, now: Instant) {
        if let MatchStatus::PointScored { reset_timer, .. } = &mut self.view.status {
            *reset_timer = self.point_started.map_or(0.0, |start| {
                (0.8 - now.duration_since(start).as_secs_f32()).max(0.0)
            });
        }
    }
}

fn player_side(side: Side) -> PlayerSide {
    match side {
        Side::Left => PlayerSide::Left,
        Side::Right => PlayerSide::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::super::NetworkBridge;
    use super::*;

    #[test]
    fn config_validates_profiles_ports_and_utf8_byte_lengths_without_dns() {
        assert!(
            NetworkConfig::new(
                "localhost:8080".into(),
                " José ".into(),
                " j@example.com ".into()
            )
            .is_ok()
        );
        assert!(NetworkConfig::new("[::1]:8080".into(), "a".into(), "a@b".into()).is_ok());
        for address in [
            "",
            "localhost",
            "localhost:0",
            "host:65536",
            "https://host:8080",
            "[::1]:x",
        ] {
            assert!(NetworkConfig::new(address.into(), "a".into(), "a@b".into()).is_err());
        }
        for nickname in [" ".into(), "x\nY".into(), "é".repeat(128)] {
            assert!(NetworkConfig::new("127.0.0.1:8080".into(), nickname, "a@b".into()).is_err());
        }
        for email in ["x", "x@", "@x", "x@y@z", "a b@x"] {
            assert!(NetworkConfig::new("127.0.0.1:8080".into(), "a".into(), email.into()).is_err());
        }
    }

    #[test]
    fn late_ack_is_not_lost_after_a_different_message_type() {
        let (_bridge, endpoint) = NetworkBridge::new();
        let config =
            NetworkConfig::new("127.0.0.1:8080".into(), "Ana".into(), "a@b".into()).unwrap();
        let mut worker = Worker::new(endpoint, config);
        let now = Instant::now();
        assert!(!worker.accept(
            Packet {
                sequence: 100,
                session_id: 3,
                message: Message::WaitingOpponent
            },
            now
        ));
        assert!(worker.accept(
            Packet {
                sequence: 1,
                session_id: 3,
                message: Message::HelloAck {
                    assigned_session_id: 3
                }
            },
            now
        ));
        assert_eq!(worker.session, 3);
    }

    #[test]
    fn abandonment_before_match_metadata_preserves_the_buffered_authoritative_score() {
        let (_bridge, endpoint) = NetworkBridge::new();
        let config = NetworkConfig::new(
            "127.0.0.1:8080".into(),
            "Ana".into(),
            "ana@example.com".into(),
        )
        .unwrap();
        let mut worker = Worker::new(endpoint, config);
        let now = Instant::now();
        for (sequence, message) in [
            (
                1,
                Message::HelloAck {
                    assigned_session_id: 77,
                },
            ),
            (
                4,
                Message::StateSync(StateSync {
                    ball_x: 450.0,
                    ball_y: 320.0,
                    paddle_left_y: 250.0,
                    paddle_right_y: 350.0,
                    score_left: 5,
                    score_right: 2,
                    status: StateStatus::InGame,
                }),
            ),
            (
                5,
                Message::GameOver {
                    winner_side: Side::Right,
                    reason: crate::protocol::EndReason::OpponentDisconnected,
                },
            ),
            (
                3,
                Message::MatchStart {
                    side: Side::Left,
                    opponent_nickname: "Bob".into(),
                },
            ),
        ] {
            assert!(worker.accept(
                Packet {
                    sequence,
                    session_id: 77,
                    message
                },
                now
            ));
        }
        assert_eq!(
            worker.view.status,
            MatchStatus::GameOver {
                winner: PlayerSide::Right
            }
        );
        assert_eq!((worker.view.score_left, worker.view.score_right), (5, 2));
        assert_eq!(worker.view.my_side, PlayerSide::Left);
        assert_eq!(
            (&*worker.view.p1_name, &*worker.view.p2_name),
            ("Ana", "Bob")
        );
        assert_eq!((worker.view.ball_x, worker.view.ball_y), (450.0, 320.0));
        assert_eq!(
            (worker.view.paddle_left_y, worker.view.paddle_right_y),
            (250.0, 350.0)
        );
    }
}
