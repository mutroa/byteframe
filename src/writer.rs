//! Packet writer that wraps any `std::io::Write` sink.

use std::io::{self, Write};

use crate::codec::{self, CodecError};
use crate::packet::Packet;

/// Wraps a `Write` sink and provides packet-level writing.
///
/// This adapter uses the protocol's encoder to serialize packets
/// into wire format and write them to the underlying sink.
///
/// # Example
///
/// ```no_run
/// use std::net::TcpStream;
/// use byteframe::writer::PacketWriter;
/// use byteframe::Packet;
///
/// let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
/// let mut writer = PacketWriter::new(stream);
///
/// writer.write_packet(&Packet::Ping).unwrap();
/// writer.write_packet(&Packet::Message("hello".into())).unwrap();
/// writer.flush().unwrap();
/// ```
pub struct PacketWriter<W> {
    writer: W,
    encode_buffer: Vec<u8>,
}

impl<W: Write> PacketWriter<W> {
    /// Create a new packet writer wrapping the given `Write` sink.
    pub fn new(writer: W) -> Self {
        Self::with_capacity(writer, 1024)
    }

    /// Create a new packet writer with a specific encode buffer capacity.
    pub fn with_capacity(writer: W, capacity: usize) -> Self {
        Self {
            writer,
            encode_buffer: Vec::with_capacity(capacity),
        }
    }

    /// Write a single packet to the stream.
    ///
    /// This method encodes the packet and writes the complete frame
    /// (header + payload) to the underlying writer.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if:
    /// - The packet payload exceeds the maximum size (65535 bytes)
    /// - The underlying write operation fails
    pub fn write_packet(&mut self, packet: &Packet) -> io::Result<()> {
        self.encode_buffer.clear(); // Clear buffer and encode packet
        codec::encode(packet, &mut self.encode_buffer).map_err(codec_to_io_error)?;
        
        self.writer.write_all(&self.encode_buffer)?; // Write the complete frame atomically
        Ok(())
    }

    /// Flush the underlying writer.
    ///
    /// This ensures all buffered data is written to the underlying sink.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Access the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Access the underlying writer mutably.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Unwrap and return the underlying writer.
    pub fn into_writer(self) -> W {
        self.writer
    }
}

/// Helper to convert codec errors to I/O errors.
fn codec_to_io_error(err: CodecError) -> io::Error {
    match err {
        CodecError::PayloadTooLarge(size) => io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("payload too large: {} bytes (max: 65535)", size),
        ),
        other => io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec;
    use crate::packet::Packet;

    #[test]
    fn writes_single_packet() {
        let mut buf = Vec::new();
        let mut writer = PacketWriter::new(&mut buf);

        writer.write_packet(&Packet::Ping).unwrap();
        writer.flush().unwrap();

        // Verify by decoding
        let packet = codec::decode(&buf).unwrap();
        assert_eq!(packet, Packet::Ping);
    }

    #[test]
    fn writes_multiple_packets() {
        let mut buf = Vec::new();
        let mut writer = PacketWriter::new(&mut buf);

        let packets = vec![
            Packet::Ping,
            Packet::Message("hello".into()),
            Packet::Data(vec![1, 2, 3]),
            Packet::Pong,
        ];

        for packet in &packets {
            writer.write_packet(packet).unwrap();
        }
        writer.flush().unwrap();

        // Verify by decoding each frame
        let mut offset = 0;
        for expected in &packets {
            let packet = codec::decode(&buf[offset..]).unwrap();
            assert_eq!(&packet, expected);

            // Calculate frame size to advance offset
            let mut frame_buf = Vec::new();
            codec::encode(expected, &mut frame_buf).unwrap();
            offset += frame_buf.len();
        }
    }

    #[test]
    fn rejects_oversized_packet() {
        let mut buf = Vec::new();
        let mut writer = PacketWriter::new(&mut buf);

        // Create a packet larger than u16::MAX
        let huge_data = vec![0u8; 70000];
        let packet = Packet::Data(huge_data);

        let err = writer.write_packet(&packet).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn encodes_correctly() {
        let mut buf = Vec::new();
        let mut writer = PacketWriter::new(&mut buf);

        let message = Packet::Message("test".into());
        writer.write_packet(&message).unwrap();

        // Manually verify the wire format
        assert_eq!(buf[0], 0xAA); // Magic byte 1
        assert_eq!(buf[1], 0x55); // Magic byte 2
        assert_eq!(buf[2], 0x03); // Opcode for Message
        assert_eq!(buf[3], 0x00); // Length high byte
        assert_eq!(buf[4], 0x04); // Length low byte (4 bytes for "test")
        // bytes[5..9] are checksum
        assert_eq!(&buf[9..13], b"test"); // Payload
    }
}
