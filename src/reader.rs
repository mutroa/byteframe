//! Packet reader that wraps any `std::io::Read` source.

use std::io::{self, Read};

use crate::framing::FrameDecoder;
use crate::packet::Packet;

/// Wraps a `Read` source and provides packet-level reading.
///
/// This adapter uses the protocol's framing decoder to parse packets
/// from a byte stream. It handles buffering internally.
///
/// # Example
///
/// ```no_run
/// use std::net::TcpStream;
/// use byteframe::reader::PacketReader;
///
/// let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
/// let mut reader = PacketReader::new(stream);
///
/// loop {
///     match reader.read_packet() {
///         Ok(packet) => println!("Received: {:?}", packet),
///         Err(e) => {
///             eprintln!("Error: {}", e);
///             break;
///         }
///     }
/// }
/// ```
pub struct PacketReader<R> {
    reader: R,
    decoder: FrameDecoder,
    read_buffer: Vec<u8>,
    packet_buffer: Vec<Packet>,
}

impl<R: Read> PacketReader<R> {
    /// Create a new packet reader wrapping the given `Read` source.
    pub fn new(reader: R) -> Self {
        Self::with_capacity(reader, 4096)
    }

    /// Create a new packet reader with a specific read buffer size.
    pub fn with_capacity(reader: R, capacity: usize) -> Self {
        Self {
            reader,
            decoder: FrameDecoder::new(),
            read_buffer: vec![0u8; capacity],
            packet_buffer: Vec::new(),
        }
    }

    /// Read one complete packet from the stream.
    ///
    /// This method blocks until a complete packet is available or an error occurs.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if:
    /// - The underlying read fails
    /// - The stream ends unexpectedly (EOF)
    /// - A packet fails checksum validation
    /// - An invalid opcode is encountered
    pub fn read_packet(&mut self) -> io::Result<Packet> {
        loop {
            // Return buffered packet if available
            if !self.packet_buffer.is_empty() {
                return Ok(self.packet_buffer.remove(0));
            }

            // Read more data from the underlying stream
            let bytes_read = self.reader.read(&mut self.read_buffer)?;

            if bytes_read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Stream closed before complete packet received",
                ));
            }

            // Feed bytes to the decoder
            let decode_result = self.decoder.decode(&self.read_buffer[..bytes_read]);

            // Check for errors (optional: you could log these instead of failing)
            if let Some(err) = decode_result.errors.first() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("framing error: {:?}", err),
                ));
            }

            // Buffer all decoded packets
            self.packet_buffer.extend(decode_result.packets);

            // Continue looping - next iteration will return first buffered packet
        }
    }

    /// Access the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    /// Access the underlying reader mutably.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Unwrap and return the underlying reader, discarding any buffered data.
    pub fn into_reader(self) -> R {
        self.reader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec;
    use crate::packet::Packet;
    use std::io::Cursor;

    fn encode_packets(packets: &[Packet]) -> Vec<u8> {
        let mut buf = Vec::new();
        for packet in packets {
            codec::encode(packet, &mut buf).unwrap();
        }
        buf
    }

    #[test]
    fn reads_single_packet() {
        let wire_data = encode_packets(&[Packet::Ping]);
        let cursor = Cursor::new(wire_data);
        let mut reader = PacketReader::new(cursor);

        let packet = reader.read_packet().unwrap();
        assert_eq!(packet, Packet::Ping);
    }

    #[test]
    fn reads_multiple_packets() {
        let packets = vec![
            Packet::Ping,
            Packet::Message("hello".into()),
            Packet::Pong,
        ];
        let wire_data = encode_packets(&packets);
        let cursor = Cursor::new(wire_data);
        let mut reader = PacketReader::new(cursor);

        for expected in &packets {
            let packet = reader.read_packet().unwrap();
            assert_eq!(&packet, expected);
        }
    }

    #[test]
    fn handles_fragmented_reads() {
        // Create a reader that returns 1 byte at a time
        struct OneByteReader {
            data: Vec<u8>,
            pos: usize,
        }

        impl Read for OneByteReader {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if self.pos >= self.data.len() {
                    return Ok(0);
                }
                if buf.is_empty() {
                    return Ok(0);
                }
                buf[0] = self.data[self.pos];
                self.pos += 1;
                Ok(1)
            }
        }

        let wire_data = encode_packets(&[Packet::Message("test".into())]);
        let one_byte_reader = OneByteReader {
            data: wire_data,
            pos: 0,
        };
        let mut reader = PacketReader::new(one_byte_reader);

        let packet = reader.read_packet().unwrap();
        assert_eq!(packet, Packet::Message("test".into()));
    }

    #[test]
    fn errors_on_eof() {
        let cursor = Cursor::new(Vec::new());
        let mut reader = PacketReader::new(cursor);

        let err = reader.read_packet().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }
}
