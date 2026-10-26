//! MyAppGameProtocol v1: codificación manual, sin dependencias ni sockets.
use std::fmt;

pub const HEADER_LEN: usize = 12;
pub const MAX_DATAGRAM_LEN: usize = HEADER_LEN + 2 + 255 + 255;
pub const MAGIC: u8 = 0x50;
pub const VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    Truncated,
    InvalidMagic,
    UnsupportedVersion,
    UnknownMessage(u16),
    LengthMismatch,
    InvalidValue(&'static str),
    InvalidUtf8,
    StringTooLong,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "paquete inválido: {self:?}")
    }
}
impl std::error::Error for ProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    HelloReq = 1,
    HelloAck = 2,
    WaitingOpponent = 3,
    MatchStart = 4,
    InputCmd = 5,
    StateSync = 6,
    GameOver = 7,
    Ping = 8,
    Pong = 9,
    Leave = 10,
}

impl TryFrom<u16> for MessageType {
    type Error = ProtocolError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::HelloReq),
            2 => Ok(Self::HelloAck),
            3 => Ok(Self::WaitingOpponent),
            4 => Ok(Self::MatchStart),
            5 => Ok(Self::InputCmd),
            6 => Ok(Self::StateSync),
            7 => Ok(Self::GameOver),
            8 => Ok(Self::Ping),
            9 => Ok(Self::Pong),
            10 => Ok(Self::Leave),
            other => Err(ProtocolError::UnknownMessage(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub message_type: MessageType,
    pub sequence: u32,
    pub session_id: u16,
    pub payload_len: u16,
}

impl Header {
    pub fn encode(self) -> [u8; HEADER_LEN] {
        let mut bytes = [0; HEADER_LEN];
        bytes[0] = MAGIC;
        bytes[1] = VERSION;
        bytes[2..4].copy_from_slice(&(self.message_type as u16).to_be_bytes());
        bytes[4..8].copy_from_slice(&self.sequence.to_be_bytes());
        bytes[8..10].copy_from_slice(&self.session_id.to_be_bytes());
        bytes[10..12].copy_from_slice(&self.payload_len.to_be_bytes());
        bytes
    }

    /// Lee solo la cabecera. `Packet::decode` comprueba también el payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < HEADER_LEN {
            return Err(ProtocolError::Truncated);
        }
        if bytes[0] != MAGIC {
            return Err(ProtocolError::InvalidMagic);
        }
        if bytes[1] != VERSION {
            return Err(ProtocolError::UnsupportedVersion);
        }
        let mut reader = Reader::new(&bytes[2..HEADER_LEN]);
        Ok(Self {
            message_type: MessageType::try_from(reader.u16()?)?,
            sequence: reader.u32()?,
            session_id: reader.u16()?,
            payload_len: reader.u16()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Side {
    Left = 1,
    Right = 2,
}
impl Side {
    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
    fn decode(value: u8) -> Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::Left),
            2 => Ok(Self::Right),
            _ => Err(ProtocolError::InvalidValue("side")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    Stop = 0,
    MoveUp = 1,
    MoveDown = 2,
}
impl Action {
    fn decode(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::Stop),
            1 => Ok(Self::MoveUp),
            2 => Ok(Self::MoveDown),
            _ => Err(ProtocolError::InvalidValue("action")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StateStatus {
    InGame = 0,
    LeftPoint = 1,
    RightPoint = 2,
    Finished = 3,
}
impl StateStatus {
    fn decode(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::InGame),
            1 => Ok(Self::LeftPoint),
            2 => Ok(Self::RightPoint),
            3 => Ok(Self::Finished),
            _ => Err(ProtocolError::InvalidValue("status")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EndReason {
    ScoreLimit = 0,
    OpponentDisconnected = 1,
}
impl EndReason {
    fn decode(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::ScoreLimit),
            1 => Ok(Self::OpponentDisconnected),
            _ => Err(ProtocolError::InvalidValue("reason")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateSync {
    pub ball_x: f32,
    pub ball_y: f32,
    pub paddle_left_y: f32,
    pub paddle_right_y: f32,
    pub score_left: u8,
    pub score_right: u8,
    pub status: StateStatus,
}
impl StateSync {
    fn validate(&self) -> Result<(), ProtocolError> {
        for (name, value, max) in [
            ("ball_x", self.ball_x, 800.0),
            ("ball_y", self.ball_y, 600.0),
            ("paddle_left_y", self.paddle_left_y, 600.0),
            ("paddle_right_y", self.paddle_right_y, 600.0),
        ] {
            if !value.is_finite() || !(0.0..=max).contains(&value) {
                return Err(ProtocolError::InvalidValue(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    HelloReq {
        nickname: String,
        email: String,
    },
    HelloAck {
        assigned_session_id: u16,
    },
    WaitingOpponent,
    MatchStart {
        side: Side,
        opponent_nickname: String,
    },
    InputCmd {
        action: Action,
    },
    StateSync(StateSync),
    GameOver {
        winner_side: Side,
        reason: EndReason,
    },
    Ping {
        token: u32,
    },
    Pong {
        token: u32,
    },
    Leave,
}

impl Message {
    pub fn message_type(&self) -> MessageType {
        match self {
            Self::HelloReq { .. } => MessageType::HelloReq,
            Self::HelloAck { .. } => MessageType::HelloAck,
            Self::WaitingOpponent => MessageType::WaitingOpponent,
            Self::MatchStart { .. } => MessageType::MatchStart,
            Self::InputCmd { .. } => MessageType::InputCmd,
            Self::StateSync(_) => MessageType::StateSync,
            Self::GameOver { .. } => MessageType::GameOver,
            Self::Ping { .. } => MessageType::Ping,
            Self::Pong { .. } => MessageType::Pong,
            Self::Leave => MessageType::Leave,
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut bytes = Vec::new();
        match self {
            Self::HelloReq { nickname, email } => {
                push_string(&mut bytes, nickname)?;
                push_string(&mut bytes, email)?;
            }
            Self::HelloAck {
                assigned_session_id,
            } => {
                if *assigned_session_id == 0 {
                    return Err(ProtocolError::InvalidValue("session_id"));
                }
                bytes.extend_from_slice(&assigned_session_id.to_be_bytes());
            }
            Self::WaitingOpponent | Self::Leave => {}
            Self::MatchStart {
                side,
                opponent_nickname,
            } => {
                bytes.push(*side as u8);
                push_string(&mut bytes, opponent_nickname)?;
            }
            Self::InputCmd { action } => bytes.push(*action as u8),
            Self::StateSync(state) => {
                state.validate()?;
                for value in [
                    state.ball_x,
                    state.ball_y,
                    state.paddle_left_y,
                    state.paddle_right_y,
                ] {
                    bytes.extend_from_slice(&value.to_be_bytes());
                }
                bytes.extend_from_slice(&[state.score_left, state.score_right, state.status as u8]);
            }
            Self::GameOver {
                winner_side,
                reason,
            } => {
                bytes.extend_from_slice(&[*winner_side as u8, *reason as u8]);
            }
            Self::Ping { token } | Self::Pong { token } => {
                bytes.extend_from_slice(&token.to_be_bytes());
            }
        }
        Ok(bytes)
    }

    fn decode_payload(kind: MessageType, bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut reader = Reader::new(bytes);
        let message = match kind {
            MessageType::HelloReq => Self::HelloReq {
                nickname: reader.string()?,
                email: reader.string()?,
            },
            MessageType::HelloAck => {
                let id = reader.u16()?;
                if id == 0 {
                    return Err(ProtocolError::InvalidValue("session_id"));
                }
                Self::HelloAck {
                    assigned_session_id: id,
                }
            }
            MessageType::WaitingOpponent => Self::WaitingOpponent,
            MessageType::MatchStart => Self::MatchStart {
                side: Side::decode(reader.u8()?)?,
                opponent_nickname: reader.string()?,
            },
            MessageType::InputCmd => Self::InputCmd {
                action: Action::decode(reader.u8()?)?,
            },
            MessageType::StateSync => {
                let state = StateSync {
                    ball_x: reader.f32()?,
                    ball_y: reader.f32()?,
                    paddle_left_y: reader.f32()?,
                    paddle_right_y: reader.f32()?,
                    score_left: reader.u8()?,
                    score_right: reader.u8()?,
                    status: StateStatus::decode(reader.u8()?)?,
                };
                state.validate()?;
                Self::StateSync(state)
            }
            MessageType::GameOver => Self::GameOver {
                winner_side: Side::decode(reader.u8()?)?,
                reason: EndReason::decode(reader.u8()?)?,
            },
            MessageType::Ping => Self::Ping {
                token: reader.u32()?,
            },
            MessageType::Pong => Self::Pong {
                token: reader.u32()?,
            },
            MessageType::Leave => Self::Leave,
        };
        if !reader.remaining.is_empty() {
            return Err(ProtocolError::LengthMismatch);
        }
        Ok(message)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub sequence: u32,
    pub session_id: u16,
    pub message: Message,
}
impl Packet {
    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        let payload = self.message.encode_payload()?;
        let header = Header {
            message_type: self.message.message_type(),
            sequence: self.sequence,
            session_id: self.session_id,
            payload_len: payload.len() as u16,
        };
        let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len());
        bytes.extend_from_slice(&header.encode());
        bytes.extend_from_slice(&payload);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let header = Header::decode(bytes)?;
        if bytes.len() != HEADER_LEN + usize::from(header.payload_len) {
            return Err(ProtocolError::LengthMismatch);
        }
        Ok(Self {
            sequence: header.sequence,
            session_id: header.session_id,
            message: Message::decode_payload(header.message_type, &bytes[HEADER_LEN..])?,
        })
    }
}

/// Aritmética serial modular: admite el paso u32::MAX -> 0 y rechaza repetidos.
pub fn sequence_is_newer(candidate: u32, previous: u32) -> bool {
    let difference = candidate.wrapping_sub(previous);
    difference != 0 && difference < (1 << 31)
}

fn push_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), ProtocolError> {
    let length = u8::try_from(value.len()).map_err(|_| ProtocolError::StringTooLong)?;
    if length == 0 {
        return Err(ProtocolError::InvalidValue("empty string"));
    }
    bytes.push(length);
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

/// Cada extracción verifica el límite antes de acceder al slice.
struct Reader<'a> {
    remaining: &'a [u8],
}
impl<'a> Reader<'a> {
    fn new(remaining: &'a [u8]) -> Self {
        Self { remaining }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], ProtocolError> {
        if self.remaining.len() < length {
            return Err(ProtocolError::Truncated);
        }
        let (value, rest) = self.remaining.split_at(length);
        self.remaining = rest;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ProtocolError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Result<f32, ProtocolError> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn string(&mut self) -> Result<String, ProtocolError> {
        let length = usize::from(self.u8()?);
        if length == 0 {
            return Err(ProtocolError::InvalidValue("empty string"));
        }
        std::str::from_utf8(self.take(length)?)
            .map(str::to_owned)
            .map_err(|_| ProtocolError::InvalidUtf8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(message: Message) -> Packet {
        Packet {
            sequence: 0x01020304,
            session_id: 0x0506,
            message,
        }
    }
    fn state() -> StateSync {
        StateSync {
            ball_x: 400.0,
            ball_y: 300.0,
            paddle_left_y: 200.0,
            paddle_right_y: 500.0,
            score_left: 3,
            score_right: 2,
            status: StateStatus::LeftPoint,
        }
    }

    #[test]
    fn header_and_input_have_exact_network_byte_order() {
        let value = packet(Message::InputCmd {
            action: Action::MoveUp,
        });
        let expected = [0x50, 1, 0, 5, 1, 2, 3, 4, 5, 6, 0, 1, 1];
        assert_eq!(value.encode().unwrap(), expected);
        assert_eq!(Packet::decode(&expected).unwrap(), value);
        assert_eq!(
            Header::decode(&expected).unwrap().encode(),
            expected[..HEADER_LEN]
        );
    }

    #[test]
    fn every_message_roundtrips_without_changing_bytes() {
        for message in [
            Message::HelloReq {
                nickname: "José".into(),
                email: "j@example.com".into(),
            },
            Message::HelloAck {
                assigned_session_id: 0x0506,
            },
            Message::WaitingOpponent,
            Message::MatchStart {
                side: Side::Right,
                opponent_nickname: "Ana".into(),
            },
            Message::InputCmd {
                action: Action::Stop,
            },
            Message::InputCmd {
                action: Action::MoveDown,
            },
            Message::StateSync(state()),
            Message::GameOver {
                winner_side: Side::Left,
                reason: EndReason::ScoreLimit,
            },
            Message::GameOver {
                winner_side: Side::Right,
                reason: EndReason::OpponentDisconnected,
            },
        ] {
            let original = packet(message);
            let bytes = original.encode().unwrap();
            let restored = Packet::decode(&bytes).unwrap();
            assert_eq!(restored, original);
            assert_eq!(restored.encode().unwrap(), bytes);
            for length in 0..bytes.len() {
                assert!(Packet::decode(&bytes[..length]).is_err());
            }
        }
    }

    #[test]
    fn state_has_exact_31_byte_layout() {
        let bytes = packet(Message::StateSync(state())).encode().unwrap();
        assert_eq!(bytes.len(), 31);
        assert_eq!(
            &bytes[12..],
            &[
                0x43, 0xc8, 0, 0, 0x43, 0x96, 0, 0, 0x43, 0x48, 0, 0, 0x43, 0xfa, 0, 0, 3, 2, 1,
            ]
        );
    }

    #[test]
    fn strings_count_utf8_bytes_and_enforce_u8_length() {
        let bytes = packet(Message::HelloReq {
            nickname: "José".into(),
            email: "a@b".into(),
        })
        .encode()
        .unwrap();
        assert_eq!(
            &bytes[12..],
            &[5, b'J', b'o', b's', 0xc3, 0xa9, 3, b'a', b'@', b'b']
        );
        for name in [String::new(), "x".repeat(256)] {
            assert!(
                packet(Message::HelloReq {
                    nickname: name,
                    email: "a@b".into()
                })
                .encode()
                .is_err()
            );
        }
        let maximum = packet(Message::HelloReq {
            nickname: "x".repeat(255),
            email: "y".repeat(255),
        });
        assert_eq!(maximum.encode().unwrap().len(), MAX_DATAGRAM_LEN);
        assert_eq!(Packet::decode(&maximum.encode().unwrap()).unwrap(), maximum);
    }

    #[test]
    fn malformed_headers_payloads_and_discriminants_are_rejected() {
        let bytes = packet(Message::InputCmd {
            action: Action::Stop,
        })
        .encode()
        .unwrap();
        for (offset, invalid) in [(0, 0), (1, 2), (3, 255), (11, 2), (12, 255)] {
            let mut bad = bytes.clone();
            bad[offset] = invalid;
            assert!(Packet::decode(&bad).is_err(), "offset {offset}");
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(Packet::decode(&extra).is_err());
        extra[11] = 2;
        assert!(Packet::decode(&extra).is_err());
        let mut bad_utf8 = packet(Message::HelloReq {
            nickname: "a".into(),
            email: "b".into(),
        })
        .encode()
        .unwrap();
        bad_utf8[13] = 0xff;
        assert_eq!(Packet::decode(&bad_utf8), Err(ProtocolError::InvalidUtf8));
        for message in [
            Message::MatchStart {
                side: Side::Left,
                opponent_nickname: "a".into(),
            },
            Message::GameOver {
                winner_side: Side::Right,
                reason: EndReason::ScoreLimit,
            },
        ] {
            let mut bad = packet(message).encode().unwrap();
            bad[12] = 0;
            assert!(Packet::decode(&bad).is_err());
        }
        let mut bad = packet(Message::GameOver {
            winner_side: Side::Left,
            reason: EndReason::ScoreLimit,
        })
        .encode()
        .unwrap();
        bad[13] = 2;
        assert!(Packet::decode(&bad).is_err());
        let mut bad = packet(Message::StateSync(state())).encode().unwrap();
        bad[30] = 4;
        assert!(Packet::decode(&bad).is_err());
        assert!(
            packet(Message::HelloAck {
                assigned_session_id: 0
            })
            .encode()
            .is_err()
        );
        let mut bad = packet(Message::HelloAck {
            assigned_session_id: 1,
        })
        .encode()
        .unwrap();
        bad[13] = 0;
        assert!(Packet::decode(&bad).is_err());
    }

    #[test]
    fn non_finite_and_out_of_field_coordinates_are_rejected() {
        for invalid in [f32::NAN, f32::INFINITY, -1.0, 801.0] {
            let mut value = state();
            value.ball_x = invalid;
            assert!(packet(Message::StateSync(value)).encode().is_err());
            let mut bytes = packet(Message::StateSync(state())).encode().unwrap();
            bytes[12..16].copy_from_slice(&invalid.to_be_bytes());
            assert!(Packet::decode(&bytes).is_err());
        }
    }

    #[test]
    fn sequence_comparison_handles_wraparound_and_duplicates() {
        assert!(sequence_is_newer(0, u32::MAX));
        assert!(sequence_is_newer(42, 40));
        assert!(!sequence_is_newer(40, 42));
        assert!(!sequence_is_newer(42, 42));
        assert!(!sequence_is_newer(u32::MAX, 0));
        assert!(!sequence_is_newer(1 << 31, 0));
    }
}
