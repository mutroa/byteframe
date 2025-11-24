pub mod checksum;
pub mod codec;
pub mod framing;
pub mod header;
pub mod packet;

// Optional I/O helpers (require std::io)
pub mod reader;
pub mod writer;

pub use checksum::fnv1a32;
pub use codec::{decode, encode, CodecError};
pub use framing::{FrameDecoder, FrameError, DecodeResult};
pub use header::{Header, HeaderError, HEADER_LEN, HEADER_MAGIC};
pub use packet::Packet;
pub use reader::PacketReader;
pub use writer::PacketWriter;


