//! Encoding and decoding helpers for wire packets.

use std::borrow::Cow;

use crate::checksum::fnv1a32;
use crate::header::{Header, HeaderError, HEADER_LEN};
use crate::packet::{Packet, OPCODE_DATA, OPCODE_MESSAGE, OPCODE_PING, OPCODE_PONG};

#[derive(Debug)]
pub enum CodecError {
    Header(HeaderError),
    FrameTooShort(usize),
    PayloadTooLarge(usize),
    PayloadLengthMismatch { declared: u16, actual: usize },
    InvalidOpcode(u8),
    InvalidUtf8(std::string::FromUtf8Error),
    ChecksumMismatch { expected: u32, actual: u32 },
}

impl From<HeaderError> for CodecError {
    fn from(err: HeaderError) -> Self {
        CodecError::Header(err)
    }
}

pub fn encode(packet: &Packet, buf: &mut Vec<u8>) -> Result<(), CodecError> {
    let payload = extract_payload(packet);
    if payload.len() > u16::MAX as usize {
        return Err(CodecError::PayloadTooLarge(payload.len()));
    }

    let length = payload.len() as u16;
    let checksum = fnv1a32(&payload);
    let header = Header::new(packet.opcode(), length, checksum);

    buf.extend_from_slice(&header.to_bytes());
    buf.extend_from_slice(&payload);
    Ok(())
}

pub fn decode(bytes: &[u8]) -> Result<Packet, CodecError> {
    if bytes.len() < HEADER_LEN {
        return Err(CodecError::FrameTooShort(bytes.len()));
    }

    let header = Header::from_bytes(&bytes[0..HEADER_LEN])?; // Step 1: Parse ONLY the header (first 9 bytes)
    let payload_len = header.length as usize; // Step 2: Use the header to find where the payload is
    if bytes.len() < HEADER_LEN + payload_len {
        return Err(CodecError::FrameTooShort(bytes.len()));
    }
    let payload = &bytes[HEADER_LEN..][..payload_len]; // Step 3: Now extract the payload (bytes after the header)
    
    decode_frame(&header, payload)
}

    

pub(crate) fn decode_frame(header: &Header, payload: &[u8]) -> Result<Packet, CodecError> {
    if payload.len() != header.length as usize {
        return Err(CodecError::PayloadLengthMismatch {
            declared: header.length,
            actual: payload.len(),
        });
    }

    let actual = fnv1a32(payload);
    if actual != header.checksum {
        return Err(CodecError::ChecksumMismatch {
            expected: header.checksum,
            actual,
        });
    }

    packet_from_opcode(header.opcode, payload)
}

fn extract_payload(packet: &Packet) -> Cow<'_, [u8]> {
    match packet {
        Packet::Ping | Packet::Pong => Cow::Borrowed(&[]),
        Packet::Message(text) => Cow::Owned(text.as_bytes().to_vec()),
        Packet::Data(bytes) => Cow::Borrowed(bytes.as_slice()),
    }
}

fn packet_from_opcode(opcode: u8, payload: &[u8]) -> Result<Packet, CodecError> {
    match opcode {
        OPCODE_PING => {
            if !payload.is_empty() {
                return Err(CodecError::PayloadLengthMismatch { declared: 0, actual: payload.len() });
            }
            Ok(Packet::Ping)
        }
        OPCODE_PONG => {
            if !payload.is_empty() {
                return Err(CodecError::PayloadLengthMismatch { declared: 0, actual: payload.len() });
            }
            Ok(Packet::Pong)
        }
        OPCODE_MESSAGE => {
            let text = String::from_utf8(payload.to_vec()).map_err(CodecError::InvalidUtf8)?;
            Ok(Packet::Message(text))
        }
        OPCODE_DATA => Ok(Packet::Data(payload.to_vec())),
        other => Err(CodecError::InvalidOpcode(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_ping_round_trip() {
        let mut buf = Vec::new();
        encode(&Packet::Ping, &mut buf).unwrap();
        let decoded = decode(&buf).unwrap();
        assert_eq!(decoded, Packet::Ping);
    }

    #[test]
    fn encode_decode_message_round_trip() {
        let packet = Packet::Message("hello".into());
        let mut buf = Vec::new();
        encode(&packet, &mut buf).unwrap();
        let decoded = decode(&buf).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn rejects_bad_checksum() {
        let packet = Packet::Message("hello".into());
        let mut buf = Vec::new();
        encode(&packet, &mut buf).unwrap();
        let last = buf.len() - 1;
        buf[last] ^= 0xFF;
        let err = decode(&buf).unwrap_err();
        assert!(matches!(err, CodecError::ChecksumMismatch { .. }));
    }

    #[test]
    fn errors_on_invalid_opcode() {
        let mut buf = Vec::new();
        let header = Header::new(0xFF, 0, fnv1a32(&[]));
        buf.extend_from_slice(&header.to_bytes());
        let err = decode(&buf).unwrap_err();
        assert!(matches!(err, CodecError::InvalidOpcode(0xFF)));
    }
}
