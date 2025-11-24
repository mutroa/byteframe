//! Streaming framing state machine that turns arbitrary byte streams into packets.

use crate::codec::{self, CodecError};
use crate::header;
use crate::packet;

#[derive(Debug, Default)]
pub struct FrameDecoder {
    header_buf: Vec<u8>,            // Collecting header bytes
    current_header: Option<header::Header>, // Parsed header, now collecting payload
    payload_buf: Vec<u8>,           // Collecting payload bytes
}

#[derive(Debug, Default)]
pub struct DecodeResult {
    pub packets: Vec<packet::Packet>,
    pub errors: Vec<FrameError>,
}

#[derive(Debug)]
pub enum FrameError {
    InvalidMagic(u16),
    Codec(CodecError),
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(&mut self, input: &[u8]) -> DecodeResult {
        let mut result = DecodeResult::default();

        for &byte in input {
            if self.current_header.is_none() { // State 1 - building header until we find a payload
                self.header_buf.push(byte); // Accumulate header bytes
                if let Some(parsed_header) = self.try_extract_header(&mut result) {
                    if parsed_header.length == 0 { // Zero-length payload (Ping/Pong)
                        self.finish_frame(parsed_header, Vec::new(), &mut result);
                    } else { // Need to read `header.length` more bytes
                        self.payload_buf.clear();
                        self.current_header = Some(parsed_header); 
                    }
                }
            } else { // State 2 - after finding payload
                let expected_len = self.current_header
                    .as_ref()
                    .expect("Failed to decode frame: header information (current_header) missing during payload read")
                    .length as usize;
                self.payload_buf.push(byte);
                if self.payload_buf.len() == expected_len { // Compare with length
                    let parsed_header = self.current_header
                        .take() // Got all payload bytes
                        .expect("Failed to complete frame: header missing after collecting payload");
                    let payload = core::mem::take(&mut self.payload_buf);
                    self.finish_frame(parsed_header, payload, &mut result);
                }
            }
        }

        result
    }

    fn try_extract_header(&mut self, result: &mut DecodeResult) -> Option<header::Header> {
        loop {
            if self.header_buf.len() < header::HEADER_LEN {
                return None;
            }

            match header::Header::from_bytes(&self.header_buf[..header::HEADER_LEN]) { // Take the first 9 bytes
                Ok(parsed_header) => { // Parse them into a Header struct
                    self.header_buf.drain(..header::HEADER_LEN); // Remove the first 9 bytes and shift everything else down
                    return Some(parsed_header);
                }
                Err(header::HeaderError::InvalidMagic(magic)) => {
                    result.errors.push(FrameError::InvalidMagic(magic));
                    self.header_buf.remove(0);
                }
                Err(header::HeaderError::ShortBuffer(_)) => unreachable!(
                    "Decoder error: Buffer length validated ({} bytes >= {} required), but header parsing still failed. Please report this bug.",
                    self.header_buf.len(),
                    header::HEADER_LEN
                ),
            }
        }
    }

    fn finish_frame(&mut self, parsed_header: header::Header, payload: Vec<u8>, result: &mut DecodeResult) {
        match codec::decode_frame(&parsed_header, &payload) {
            Ok(decoded_packet) => result.packets.push(decoded_packet),
            Err(err) => result.errors.push(FrameError::Codec(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec;
    use crate::packet::Packet;

    fn encode(input_packet: &Packet) -> Vec<u8> {
        let mut buf = Vec::new();
        codec::encode(input_packet, &mut buf).unwrap();
        buf
    }

    #[test]
    fn decodes_across_partial_chunks() {
        let mut stream = Vec::new();
        stream.extend_from_slice(&encode(&packet::Packet::Ping));
        stream.extend_from_slice(&encode(&packet::Packet::Message("hi".into())));

        let mut decoder = FrameDecoder::new();
        let mut packets = Vec::new();

        for chunk in stream.chunks(3) {
            let output = decoder.decode(chunk);
            assert!(output.errors.is_empty());
            packets.extend(output.packets);
        }

        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0], packet::Packet::Ping);
        assert_eq!(packets[1], packet::Packet::Message("hi".into()));
    }

    #[test]
    fn resyncs_after_invalid_header() {
        let mut corrupted = encode(&packet::Packet::Ping);
        corrupted[0] ^= 0xFF; // break the magic constant
        let mut stream = corrupted.clone();
        stream.extend_from_slice(&encode(&packet::Packet::Pong));

        let mut decoder = FrameDecoder::new();
        let output = decoder.decode(&stream);
        assert!(output.packets.contains(&packet::Packet::Pong));
        assert!(output.errors.iter().any(|err| matches!(err, FrameError::InvalidMagic(_))));
    }

    #[test]
    fn detects_checksum_failure_and_continues() {
        let mut stream = encode(&packet::Packet::Message("hello".into()));
        let last = stream.len() - 1;
        stream[last] ^= 0xFF;
        stream.extend_from_slice(&encode(&packet::Packet::Data(vec![1, 2, 3])));

        let mut decoder = FrameDecoder::new();
        let output = decoder.decode(&stream);

        assert!(output.packets.contains(&packet::Packet::Data(vec![1, 2, 3])));
        assert!(output
            .errors
            .iter()
            .any(|err| matches!(err, FrameError::Codec(CodecError::ChecksumMismatch { .. }))));
    }
}
