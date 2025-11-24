//! High-level packet definitions.

/// Opcodes assigned to each packet variant.
pub const OPCODE_PING: u8 = 0x01;
pub const OPCODE_PONG: u8 = 0x02;
pub const OPCODE_MESSAGE: u8 = 0x03;
pub const OPCODE_DATA: u8 = 0x04;

/// Binary packets supported by the protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Ping,
    Pong,
    Message(String),
    Data(Vec<u8>),
}

impl Packet {
    pub fn opcode(&self) -> u8 {
        match self {
            Packet::Ping => OPCODE_PING,
            Packet::Pong => OPCODE_PONG,
            Packet::Message(_) => OPCODE_MESSAGE,
            Packet::Data(_) => OPCODE_DATA,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_matches_variant() {
        assert_eq!(Packet::Ping.opcode(), OPCODE_PING);
        assert_eq!(Packet::Pong.opcode(), OPCODE_PONG);
        assert_eq!(Packet::Message(String::new()).opcode(), OPCODE_MESSAGE);
        assert_eq!(Packet::Data(vec![]).opcode(), OPCODE_DATA);
    }
}
