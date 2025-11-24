//! Header definition and serialization helpers.

/// Magic value that prefixes every header.
pub const HEADER_MAGIC: u16 = 0xAA55;
/// Total number of bytes taken by the header.
pub const HEADER_LEN: usize = 9;

/// Wire header for every packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub magic: u16,
    pub opcode: u8,
    pub length: u16,
    pub checksum: u32,
}

impl Header {
    /// Build a header using the protocol's fixed magic value.
    pub fn new(opcode: u8, length: u16, checksum: u32) -> Self {
        Self {
            magic: HEADER_MAGIC,
            opcode,
            length,
            checksum,
        }
    }

    /// Serialize the header into network byte order.
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0u8; HEADER_LEN];

        // Magic (2 bytes)
        let magic_bytes: [u8; 2] = self.magic.to_be_bytes();
        bytes[0] = magic_bytes[0];
        bytes[1] = magic_bytes[1];
        
        // Opcode (1 byte)
        bytes[2] = self.opcode;
        
        // Length (2 bytes)
        let length_bytes: [u8; 2] = self.length.to_be_bytes();
        bytes[3] = length_bytes[0];
        bytes[4] = length_bytes[1];
        
        // Checksum (4 bytes)
        let checksum_bytes: [u8; 4] = self.checksum.to_be_bytes();
        bytes[5] = checksum_bytes[0];
        bytes[6] = checksum_bytes[1];
        bytes[7] = checksum_bytes[2];
        bytes[8] = checksum_bytes[3];

        bytes
    }

    /// Deserialize a header from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HeaderError> {
        if bytes.len() < HEADER_LEN {
            return Err(HeaderError::ShortBuffer(bytes.len()));
        }

        let magic = u16::from_be_bytes([bytes[0], bytes[1]]);
        if magic != HEADER_MAGIC {
            return Err(HeaderError::InvalidMagic(magic));
        }

        let opcode = bytes[2];
        let length = u16::from_be_bytes([bytes[3], bytes[4]]);
        let checksum = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);

        Ok(Self {
            magic,
            opcode,
            length,
            checksum,
        })
    }
}

/// Errors while parsing the header from bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    ShortBuffer(usize),
    InvalidMagic(u16),
}

impl core::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HeaderError::ShortBuffer(len) => write!(f, "buffer length {len} < header size {}", HEADER_LEN),
            HeaderError::InvalidMagic(value) => write!(f, "invalid header magic 0x{value:04X}"),
        }
    }
}

impl std::error::Error for HeaderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_header_bytes() {
        let header = Header::new(0x10, 42, 0xDEADBEEF);
        let bytes = header.to_bytes();
        let decoded = Header::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.magic, HEADER_MAGIC);
        assert_eq!(decoded.opcode, 0x10);
        assert_eq!(decoded.length, 42);
        assert_eq!(decoded.checksum, 0xDEADBEEF);
    }

    #[test]
    fn rejects_wrong_magic() {
        let mut bytes = Header::new(1, 0, 0).to_bytes();
        bytes[0] ^= 0xFF;
        let err = Header::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, HeaderError::InvalidMagic(_)));
    }

    #[test]
    fn detects_short_buffer() {
        let bytes = [0u8; 4];
        let err = Header::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, HeaderError::ShortBuffer(4)));
    }
}
