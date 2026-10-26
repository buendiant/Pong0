//! Un receptor UDP central y un hilo con estado propio por pareja.
//! La GUI no interviene: este módulo solo depende de la biblioteca estándar.
use crate::game::Game;
use crate::protocol::{
    Action, EndReason, MAX_DATAGRAM_LEN, Message, Packet, Side, StateSync, sequence_is_newer,
};
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub tick_interval: Duration,
    pub session_timeout: Duration,
    pub input_timeout: Duration,
    pub control_interval: Duration,
    pub terminal_retention: Duration,
    pub max_sessions: usize,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_nanos(16_666_667),
            session_timeout: Duration::from_secs(10),
            input_timeout: Duration::from_millis(250),
            control_interval: Duration::from_millis(500),
            terminal_retention: Duration::from_secs(5),
            max_sessions: 256,
        }
    }
}
impl ServerConfig {
    fn validate(&self) -> io::Result<()> {
        if self.tick_interval.is_zero()
            || self.tick_interval > Duration::from_millis(100)
            || self.input_timeout.is_zero()
            || self.input_timeout > self.session_timeout
            || self.control_interval.is_zero()
            || self.terminal_retention.is_zero()
            || self.max_sessions == 0
            || self.max_sessions > u16::MAX as usize
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "configuración de servidor inválida",
            ));
        }
        Ok(())
    }
}

struct Logger(Mutex<File>);
impl Logger {
    fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self(Mutex::new(
            OpenOptions::new().create(true).append(true).open(path)?,
        )))
    }
    fn log(&self, event: impl AsRef<str>) {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        // Un mismo candado conserva el orden de las líneas de todos los hilos.
        let mut file = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let line = format!("{time} {}", event.as_ref());
        let _ = writeln!(io::stdout().lock(), "{line}");
        if let Err(error) = writeln!(file, "{line}") {
            let _ = writeln!(io::stderr().lock(), "Error escribiendo log: {error}");
        }
    }
}

struct Wire {
    socket: Arc<UdpSocket>,
    logger: Logger,
    sequence: AtomicU32,
}
impl Wire {
    fn send(&self, id: u16, address: SocketAddr, message: Message) {
        let kind = message.message_type();
        let packet = Packet {
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            session_id: id,
            message,
        };
        match packet.encode() {
            Ok(bytes) => match self.socket.send_to(&bytes, address) {
                Ok(sent) if sent == bytes.len() => self.logger.log(format!(
                    "TX destino={address} sesión={id} tipo={kind:?} bytes={sent} datos={bytes:?}"
                )),
                Ok(sent) => self
                    .logger
                    .log(format!("ERROR TX parcial destino={address} bytes={sent}")),
                Err(error) => self
                    .logger
                    .log(format!("ERROR TX destino={address}: {error}")),
            },
            Err(error) => self
                .logger
                .log(format!("ERROR serializando {kind:?}: {error}")),
        }
    }
}

#[derive(Clone)]
enum Phase {
    Waiting,
    Playing {
        match_id: u64,
        side: Side,
        opponent: String,
    },
    Finished {
        result: Message,
        side: Side,
        opponent: String,
        final_state: Option<StateSync>,
        expires: Instant,
        next_send: Instant,
    },
}
#[derive(Clone)]
struct Session {
    id: u16,
    address: SocketAddr,
    nickname: String,
    email: String,
    last_seen: Instant,
    last_client_sequence: Option<u32>,
    phase: Phase,
}

struct Activity {
    side: Side,
    action: Option<Action>,
    received_at: Instant,
}
struct MatchFinished {
    match_id: u64,
    sessions: [u16; 2],
    result: Message,
    final_state: Option<StateSync>,
}
struct RunningMatch {
    commands: SyncSender<Activity>,
    leaving: Arc<[AtomicBool; 2]>,
    handle: JoinHandle<()>,
}

struct MatchControl {
    commands: Receiver<Activity>,
    leaving: Arc<[AtomicBool; 2]>,
}

pub struct PongServer {
    wire: Arc<Wire>,
    config: ServerConfig,
    sessions: HashMap<u16, Session>,
    addresses: HashMap<SocketAddr, u16>,
    waiting: VecDeque<u16>,
    matches: HashMap<u64, RunningMatch>,
    completed_tx: Sender<MatchFinished>,
    completed_rx: Receiver<MatchFinished>,
    next_session: u16,
    next_match: u64,
}

impl PongServer {
    pub fn bind(
        address: impl ToSocketAddrs,
        log: impl AsRef<Path>,
        config: ServerConfig,
    ) -> io::Result<Self> {
        config.validate()?;
        let logger = Logger::open(log)?;
        let socket = Arc::new(UdpSocket::bind(address).inspect_err(|error| {
            logger.log(format!("ERROR enlazando socket UDP: {error}"));
        })?);
        socket.set_nonblocking(true).inspect_err(|error| {
            logger.log(format!("ERROR configurando socket UDP: {error}"));
        })?;
        logger.log(format!(
            "Servidor Pong iniciado en {}",
            socket.local_addr()?
        ));
        let (completed_tx, completed_rx) = mpsc::channel();
        Ok(Self {
            wire: Arc::new(Wire {
                socket,
                logger,
                sequence: AtomicU32::new(1),
            }),
            config,
            sessions: HashMap::new(),
            addresses: HashMap::new(),
            waiting: VecDeque::new(),
            matches: HashMap::new(),
            completed_tx,
            completed_rx,
            next_session: 1,
            next_match: 1,
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.wire.socket.local_addr()
    }

    /// `stop` permite detener y unir todos los hilos durante pruebas o cierre del host.
    pub fn run(mut self, stop: Arc<AtomicBool>) -> io::Result<()> {
        let result = self.run_loop(&stop);
        stop.store(true, Ordering::Relaxed);
        for (_, running) in self.matches.drain() {
            if running.handle.join().is_err() {
                self.wire
                    .logger
                    .log("ERROR hilo de partida terminó con panic");
            }
        }
        self.wire
            .logger
            .log("Servidor detenido; hilos de partidas cerrados");
        result
    }

    fn run_loop(&mut self, stop: &Arc<AtomicBool>) -> io::Result<()> {
        // Buffer completo de UDP: nunca interpretar como válido un datagrama truncado.
        let mut buffer = [0u8; 65_535];
        while !stop.load(Ordering::Relaxed) {
            self.finish_matches();
            self.expire_sessions();
            // Límite por vuelta: el tráfico entrante no retrasa limpieza y cierre.
            for _ in 0..64 {
                match self.wire.socket.recv_from(&mut buffer) {
                    Ok((length, address)) => {
                        if length > MAX_DATAGRAM_LEN {
                            self.wire.logger.log(format!(
                                "RX RECHAZADO origen={address} bytes={length}: demasiado grande"
                            ));
                            continue;
                        }
                        let bytes = &buffer[..length];
                        match Packet::decode(bytes) {
                            Ok(packet) => {
                                self.wire.logger.log(format!("RX origen={address} sesión={} tipo={:?} bytes={length} datos={bytes:?}",
                                    packet.session_id, packet.message.message_type()));
                                self.handle_packet(address, packet, stop);
                            }
                            Err(error) => self.wire.logger.log(format!("RX RECHAZADO origen={address} bytes={length} datos={bytes:?}: {error}")),
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::Interrupted | io::ErrorKind::ConnectionReset
                        ) =>
                    {
                        self.wire
                            .logger
                            .log(format!("ERROR RX recuperable: {error}"));
                        break;
                    }
                    Err(error) => {
                        self.wire.logger.log(format!("ERROR RX: {error}"));
                        return Err(error);
                    }
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    fn handle_packet(&mut self, address: SocketAddr, packet: Packet, stop: &Arc<AtomicBool>) {
        match packet.message {
            Message::HelloReq { nickname, email } => {
                self.register(address, packet.session_id, nickname, email, stop);
            }
            Message::InputCmd { action } => {
                if !self.accept_client_packet(address, packet.session_id, packet.sequence) {
                    return;
                }
                let session = &self.sessions[&packet.session_id];
                match &session.phase {
                    Phase::Playing { match_id, side, .. } => {
                        if let Some(running) = self.matches.get(match_id) {
                            let now = Instant::now();
                            match running.commands.try_send(Activity {
                                side: *side,
                                action: Some(action),
                                received_at: now,
                            }) {
                                Ok(()) => {}
                                Err(TrySendError::Full(_)) => self.wire.logger.log(format!(
                                    "INPUT descartado: cola llena partida={match_id}"
                                )),
                                Err(TrySendError::Disconnected(_)) => {}
                            }
                        }
                    }
                    Phase::Waiting => self
                        .wire
                        .send(session.id, address, Message::WaitingOpponent),
                    Phase::Finished { result, .. } => {
                        self.wire.send(session.id, address, result.clone())
                    }
                }
            }
            Message::Ping { token } => {
                if !self.accept_client_packet(address, packet.session_id, packet.sequence) {
                    return;
                }
                let session = &self.sessions[&packet.session_id];
                // Ping mantiene viva la sesión, pero no renueva una intención de pala.
                if let Phase::Playing { match_id, side, .. } = &session.phase
                    && let Some(running) = self.matches.get(match_id)
                {
                    let _ = running.commands.try_send(Activity {
                        side: *side,
                        action: None,
                        received_at: session.last_seen,
                    });
                }
                self.wire.send(session.id, address, Message::Pong { token });
            }
            Message::Leave => {
                if !self.accept_client_packet(address, packet.session_id, packet.sequence) {
                    return;
                }
                self.wire.logger.log(format!(
                    "ABANDONO sesión={} origen={address}",
                    packet.session_id
                ));
                match &self.sessions[&packet.session_id].phase {
                    Phase::Playing { match_id, side, .. } => {
                        if let Some(running) = self.matches.get(match_id) {
                            // Una bandera separada de la cola acotada no pierde el cierre
                            // incluso si el cliente llenó antes la cola con movimientos.
                            running.leaving[*side as usize - 1].store(true, Ordering::Release);
                        }
                    }
                    Phase::Waiting | Phase::Finished { .. } => {
                        self.remove_session(packet.session_id)
                    }
                }
            }
            other => self.wire.logger.log(format!(
                "RECHAZADO mensaje exclusivo del servidor: {:?} origen={address}",
                other.message_type()
            )),
        }
    }

    /// Las peticiones de una sesión comparten un único espacio de secuencias.
    fn accept_client_packet(&mut self, address: SocketAddr, id: u16, sequence: u32) -> bool {
        let Some(session) = self.sessions.get_mut(&id) else {
            self.wire.logger.log(format!(
                "RECHAZADO sesión desconocida={id} origen={address}"
            ));
            return false;
        };
        if session.address != address {
            self.wire
                .logger
                .log(format!("RECHAZADO origen no corresponde a sesión={id}"));
            return false;
        }
        if session
            .last_client_sequence
            .is_some_and(|previous| !sequence_is_newer(sequence, previous))
        {
            self.wire.logger.log(format!(
                "IGNORADO comando duplicado/fuera de orden sesión={id} seq={sequence}"
            ));
            return false;
        }
        session.last_client_sequence = Some(sequence);
        session.last_seen = Instant::now();
        true
    }

    fn remove_session(&mut self, id: u16) {
        if let Some(session) = self.sessions.remove(&id) {
            self.addresses.remove(&session.address);
            self.waiting.retain(|waiting| *waiting != id);
            self.wire
                .logger
                .log(format!("SESIÓN CERRADA id={id} origen={}", session.address));
        }
    }

    fn register(
        &mut self,
        address: SocketAddr,
        claimed_id: u16,
        nickname: String,
        email: String,
        stop: &Arc<AtomicBool>,
    ) {
        if nickname.trim().is_empty()
            || email.trim().is_empty()
            || nickname.chars().any(char::is_control)
            || email.chars().any(char::is_control)
        {
            self.wire
                .logger
                .log(format!("RECHAZADO perfil inválido origen={address}"));
            return;
        }
        if let Some(id) = self.addresses.get(&address).copied() {
            let session = self
                .sessions
                .get_mut(&id)
                .expect("índice de direcciones consistente");
            if (claimed_id != 0 && claimed_id != id)
                || session.nickname != nickname
                || session.email != email
            {
                self.wire
                    .logger
                    .log(format!("RECHAZADO cambio de identidad en sesión={id}"));
                return;
            }
            session.last_seen = Instant::now();
            self.wire.send(
                id,
                address,
                Message::HelloAck {
                    assigned_session_id: id,
                },
            );
            match &session.phase {
                Phase::Waiting => self.wire.send(id, address, Message::WaitingOpponent),
                Phase::Playing {
                    match_id,
                    side,
                    opponent,
                } => {
                    self.wire.send(
                        id,
                        address,
                        Message::MatchStart {
                            side: *side,
                            opponent_nickname: opponent.clone(),
                        },
                    );
                    if let Some(running) = self.matches.get(match_id) {
                        let _ = running.commands.try_send(Activity {
                            side: *side,
                            action: None,
                            received_at: session.last_seen,
                        });
                    }
                }
                Phase::Finished {
                    result,
                    side,
                    opponent,
                    final_state,
                    ..
                } => {
                    // Si se perdió MATCH_START, recuperar lado/nombres sigue siendo
                    // necesario para presentar correctamente el resultado terminal.
                    self.wire.send(
                        id,
                        address,
                        Message::MatchStart {
                            side: *side,
                            opponent_nickname: opponent.clone(),
                        },
                    );
                    if let Some(state) = final_state {
                        self.wire
                            .send(id, address, Message::StateSync(state.clone()));
                    }
                    self.wire.send(id, address, result.clone());
                }
            }
            return;
        }
        if claimed_id != 0 || self.sessions.len() >= self.config.max_sessions {
            self.wire.logger.log(format!(
                "RECHAZADO registro sin capacidad o sesión inválida origen={address}"
            ));
            return;
        }
        let Some(id) = self.allocate_session() else {
            return;
        };
        self.sessions.insert(
            id,
            Session {
                id,
                address,
                nickname,
                email,
                last_seen: Instant::now(),
                last_client_sequence: None,
                phase: Phase::Waiting,
            },
        );
        self.addresses.insert(address, id);
        self.waiting.push_back(id);
        self.wire
            .logger
            .log(format!("REGISTRO sesión={id} origen={address}"));
        self.wire.send(
            id,
            address,
            Message::HelloAck {
                assigned_session_id: id,
            },
        );
        self.wire.send(id, address, Message::WaitingOpponent);
        self.pair_waiting(stop);
    }

    fn allocate_session(&mut self) -> Option<u16> {
        for _ in 0..u16::MAX {
            let id = self.next_session;
            self.next_session = if id == u16::MAX { 1 } else { id + 1 };
            if !self.sessions.contains_key(&id) {
                return Some(id);
            }
        }
        None
    }

    fn pair_waiting(&mut self, stop: &Arc<AtomicBool>) {
        while self.waiting.len() >= 2 {
            let left = self.waiting.pop_front().expect("cola no vacía");
            let right = self.waiting.pop_front().expect("cola no vacía");
            let players = [self.sessions[&left].clone(), self.sessions[&right].clone()];
            let match_id = self.next_match;
            self.next_match += 1;
            let (commands, receiver) = mpsc::sync_channel(128);
            let leaving = Arc::new([AtomicBool::new(false), AtomicBool::new(false)]);
            let control = MatchControl {
                commands: receiver,
                leaving: Arc::clone(&leaving),
            };
            let wire = Arc::clone(&self.wire);
            let config = self.config.clone();
            let finished = self.completed_tx.clone();
            let stopping = Arc::clone(stop);
            let roster = players.clone();
            match thread::Builder::new()
                .name(format!("pong-match-{match_id}"))
                .spawn(move || {
                    run_match(match_id, roster, control, wire, config, stopping, finished);
                }) {
                Ok(handle) => {
                    for (index, id) in [left, right].into_iter().enumerate() {
                        self.sessions.get_mut(&id).expect("sesión registrada").phase =
                            Phase::Playing {
                                match_id,
                                side: if index == 0 { Side::Left } else { Side::Right },
                                opponent: players[1 - index].nickname.clone(),
                            };
                    }
                    self.matches.insert(
                        match_id,
                        RunningMatch {
                            commands,
                            leaving,
                            handle,
                        },
                    );
                    self.wire.logger.log(format!(
                        "EMPAREJAMIENTO partida={match_id} izquierda={left} derecha={right}"
                    ));
                }
                Err(error) => {
                    self.waiting.push_front(right);
                    self.waiting.push_front(left);
                    self.wire
                        .logger
                        .log(format!("ERROR creando hilo partida={match_id}: {error}"));
                    break;
                }
            }
        }
    }

    fn finish_matches(&mut self) {
        while let Ok(finished) = self.completed_rx.try_recv() {
            if let Some(running) = self.matches.remove(&finished.match_id)
                && running.handle.join().is_err()
            {
                self.wire.logger.log("ERROR cerrando hilo de partida");
            }
            let now = Instant::now();
            for id in finished.sessions {
                if let Some(session) = self.sessions.get_mut(&id) {
                    let Phase::Playing { side, opponent, .. } = &session.phase else {
                        continue;
                    };
                    session.phase = Phase::Finished {
                        result: finished.result.clone(),
                        side: *side,
                        opponent: opponent.clone(),
                        final_state: finished.final_state.clone(),
                        expires: now + self.config.terminal_retention,
                        next_send: now + self.config.control_interval,
                    };
                }
            }
        }
    }

    fn expire_sessions(&mut self) {
        let now = Instant::now();
        let mut expired = Vec::new();
        for (&id, session) in &mut self.sessions {
            match &mut session.phase {
                Phase::Waiting
                    if now.duration_since(session.last_seen) >= self.config.session_timeout =>
                {
                    expired.push(id)
                }
                Phase::Finished { expires, .. } if now >= *expires => expired.push(id),
                Phase::Finished {
                    result,
                    final_state,
                    next_send,
                    ..
                } if now >= *next_send => {
                    if let Some(state) = final_state {
                        self.wire
                            .send(id, session.address, Message::StateSync(state.clone()));
                    }
                    self.wire.send(id, session.address, result.clone());
                    *next_send = now + self.config.control_interval;
                }
                _ => {}
            }
        }
        for id in expired {
            if let Some(session) = self.sessions.remove(&id) {
                self.addresses.remove(&session.address);
                self.wire
                    .logger
                    .log(format!("SESIÓN CERRADA id={id} origen={}", session.address));
            }
        }
        self.waiting.retain(|id| {
            self.sessions
                .get(id)
                .is_some_and(|session| matches!(session.phase, Phase::Waiting))
        });
    }
}

fn send_start(wire: &Wire, players: &[Session; 2]) {
    for (index, player) in players.iter().enumerate() {
        wire.send(
            player.id,
            player.address,
            Message::MatchStart {
                side: if index == 0 { Side::Left } else { Side::Right },
                opponent_nickname: players[1 - index].nickname.clone(),
            },
        );
    }
}

fn run_match(
    match_id: u64,
    players: [Session; 2],
    control: MatchControl,
    wire: Arc<Wire>,
    config: ServerConfig,
    stop: Arc<AtomicBool>,
    finished: Sender<MatchFinished>,
) {
    let mut game = Game::new();
    let now = Instant::now();
    let mut last_seen = [now; 2];
    let mut last_input = [now; 2];
    let mut next_tick = now + config.tick_interval;
    let mut next_control = now + config.control_interval;
    send_start(&wire, &players);
    let result = loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if let Some(departed) = control
            .leaving
            .iter()
            .position(|flag| flag.load(Ordering::Acquire))
        {
            break Message::GameOver {
                winner_side: if departed == 0 {
                    Side::Right
                } else {
                    Side::Left
                },
                reason: EndReason::OpponentDisconnected,
            };
        }
        let now = Instant::now();
        if now >= next_tick {
            if let Some(disconnected) = last_seen
                .iter()
                .position(|seen| now.duration_since(*seen) >= config.session_timeout)
            {
                break Message::GameOver {
                    winner_side: if disconnected == 0 {
                        Side::Right
                    } else {
                        Side::Left
                    },
                    reason: EndReason::OpponentDisconnected,
                };
            }
            for (index, side) in [Side::Left, Side::Right].into_iter().enumerate() {
                if now.duration_since(last_input[index]) >= config.input_timeout {
                    game.set_input(side, Action::Stop);
                }
            }
            let old = game.snapshot();
            game.advance(config.tick_interval);
            let snapshot = game.snapshot();
            if snapshot.score_left != old.score_left || snapshot.score_right != old.score_right {
                wire.logger.log(format!(
                    "GOL partida={match_id} marcador={}:{}",
                    snapshot.score_left, snapshot.score_right
                ));
            }
            for player in &players {
                wire.send(
                    player.id,
                    player.address,
                    Message::StateSync(snapshot.clone()),
                );
            }
            if let Some(winner_side) = game.winner() {
                break Message::GameOver {
                    winner_side,
                    reason: EndReason::ScoreLimit,
                };
            }
            // Mantener la cadencia sin acumular la latencia de cada despertar.
            // Si el host pierde varios ticks, omitirlos en lugar de crear una ráfaga.
            next_tick += config.tick_interval;
            if next_tick <= now {
                next_tick = now + config.tick_interval;
            }
            if now >= next_control {
                send_start(&wire, &players);
                next_control = now + config.control_interval;
            }
        }
        match control
            .commands
            .recv_timeout(next_tick.saturating_duration_since(Instant::now()))
        {
            Ok(activity) => {
                let index = activity.side as usize - 1;
                last_seen[index] = activity.received_at;
                if let Some(action) = activity.action {
                    last_input[index] = activity.received_at;
                    game.set_input(activity.side, action);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    };
    wire.logger
        .log(format!("FIN partida={match_id} resultado={result:?}"));
    let final_state = if matches!(
        &result,
        Message::GameOver {
            reason: EndReason::ScoreLimit,
            ..
        }
    ) {
        Some(game.snapshot())
    } else {
        None
    };
    for player in &players {
        wire.send(player.id, player.address, result.clone());
    }
    let _ = finished.send(MatchFinished {
        match_id,
        sessions: [players[0].id, players[1].id],
        result,
        final_state,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finished_score_snapshot_is_replayed_with_metadata_without_extending_retention() {
        let log =
            std::env::temp_dir().join(format!("pong-terminal-replay-{}.log", std::process::id()));
        let mut server = PongServer::bind("127.0.0.1:0", &log, ServerConfig::default()).unwrap();
        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client.connect(server.local_addr().unwrap()).unwrap();
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let address = client.local_addr().unwrap();
        server.sessions.insert(
            1,
            Session {
                id: 1,
                address,
                nickname: "Ana".into(),
                email: "ana@example.com".into(),
                last_seen: Instant::now(),
                last_client_sequence: None,
                phase: Phase::Playing {
                    match_id: 7,
                    side: Side::Left,
                    opponent: "Bob".into(),
                },
            },
        );
        server.addresses.insert(address, 1);
        let snapshot = StateSync {
            ball_x: 400.0,
            ball_y: 300.0,
            paddle_left_y: 200.0,
            paddle_right_y: 350.0,
            score_left: 7,
            score_right: 2,
            status: crate::protocol::StateStatus::Finished,
        };
        let result = Message::GameOver {
            winner_side: Side::Left,
            reason: EndReason::ScoreLimit,
        };
        server
            .completed_tx
            .send(MatchFinished {
                match_id: 7,
                sessions: [1, 2],
                result: result.clone(),
                final_state: Some(snapshot.clone()),
            })
            .unwrap();
        server.finish_matches();
        let expires_before = match &server.sessions[&1].phase {
            Phase::Finished { expires, .. } => *expires,
            _ => panic!("la sesión debía quedar terminada"),
        };
        server.register(
            address,
            0,
            "Ana".into(),
            "ana@example.com".into(),
            &Arc::new(AtomicBool::new(false)),
        );
        let mut buffer = [0; 1024];
        for expected in [
            Message::HelloAck {
                assigned_session_id: 1,
            },
            Message::MatchStart {
                side: Side::Left,
                opponent_nickname: "Bob".into(),
            },
            Message::StateSync(snapshot.clone()),
            result.clone(),
        ] {
            let length = client.recv(&mut buffer).unwrap();
            let packet = Packet::decode(&buffer[..length]).unwrap();
            assert_eq!(packet.session_id, 1);
            assert_eq!(packet.message, expected);
        }
        if let Phase::Finished {
            expires, next_send, ..
        } = &mut server.sessions.get_mut(&1).unwrap().phase
        {
            assert_eq!(*expires, expires_before);
            *next_send = Instant::now();
        }
        server.expire_sessions();
        for expected in [Message::StateSync(snapshot), result] {
            let length = client.recv(&mut buffer).unwrap();
            assert_eq!(Packet::decode(&buffer[..length]).unwrap().message, expected);
        }
        if let Phase::Finished { expires, .. } = &mut server.sessions.get_mut(&1).unwrap().phase {
            assert_eq!(*expires, expires_before);
            *expires = Instant::now();
        }
        server.expire_sessions();
        assert!(!server.sessions.contains_key(&1));
        assert!(!server.addresses.contains_key(&address));
        drop(server);
        let _ = std::fs::remove_file(log);
    }

    #[test]
    fn leave_is_preserved_when_the_match_input_queue_is_full() {
        let log = std::env::temp_dir().join(format!("pong-leave-full-{}.log", std::process::id()));
        let mut server = PongServer::bind("127.0.0.1:0", &log, ServerConfig::default()).unwrap();
        let address: SocketAddr = "127.0.0.1:32123".parse().unwrap();
        server.sessions.insert(
            1,
            Session {
                id: 1,
                address,
                nickname: "Ana".into(),
                email: "ana@example.com".into(),
                last_seen: Instant::now(),
                last_client_sequence: None,
                phase: Phase::Playing {
                    match_id: 7,
                    side: Side::Right,
                    opponent: "Bob".into(),
                },
            },
        );
        server.addresses.insert(address, 1);
        let (commands, _receiver) = mpsc::sync_channel(128);
        for _ in 0..128 {
            commands
                .try_send(Activity {
                    side: Side::Right,
                    action: Some(Action::MoveUp),
                    received_at: Instant::now(),
                })
                .unwrap();
        }
        let leaving = Arc::new([AtomicBool::new(false), AtomicBool::new(false)]);
        server.matches.insert(
            7,
            RunningMatch {
                commands,
                leaving: Arc::clone(&leaving),
                handle: thread::spawn(|| {}),
            },
        );
        server.handle_packet(
            address,
            Packet {
                sequence: 1,
                session_id: 1,
                message: Message::Leave,
            },
            &Arc::new(AtomicBool::new(false)),
        );
        assert!(!leaving[0].load(Ordering::Acquire));
        assert!(leaving[1].load(Ordering::Acquire));
        assert_eq!(server.sessions[&1].last_client_sequence, Some(1));
        drop(server);
        let _ = std::fs::remove_file(log);
    }
}
