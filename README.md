# ByteFrame

A minimal, `std`-only binary packet protocol library in Rust.

## Overview

ByteFrame is a lightweight binary framing protocol library built from scratch. It's designed to be:

- **I/O-agnostic**: Core protocol has no I/O dependencies
- **Robust**: Handles partial reads, corruption, and resynchronization
- **Type-safe**: Leverages Rust's type system for correctness
- **Well-tested**: Comprehensive test coverage
- **Fast**: Zero-copy where possible, efficient state machine

## Features

**Binary wire protocol** with 9-byte header (magic, opcode, length, checksum)
**FNV-1a checksumming** for corruption detection
**Streaming frame decoder** handles fragmented/multiple packets
**Type-safe packet enum** (Ping, Pong, Message, Data)
**Optional I/O helpers** for `std::io::Read` and `std::io::Write`
**No external dependencies** (pure `std`)

## Wire Format

Every packet on the wire consists of a 9-byte header followed by the payload:

```
┌──────────────┬───────────┐
│   Header     │  Payload  │
│   (9 bytes)  │ (N bytes) │
└──────────────┴───────────┘

Header Layout (big-endian):
┌──────┬────────┬────────┬──────────┐
│ 0xAA │   op   │ length │ checksum │
│ 0x55 │ code   │ u16    │   u32    │
│  u16 │   u8   │        │          │
└──────┴────────┴────────┴──────────┘
  0-1     2      3-4       5-8
```

**Fields:**
- `magic`: Always `0xAA55` (sync marker)
- `opcode`: Packet type (0x01 = Ping, 0x02 = Pong, 0x03 = Message, 0x04 = Data)
- `length`: Payload size in bytes (0-65535)
- `checksum`: FNV-1a 32-bit hash of the payload

## Examples

Run the echo server/client example:

```bash
# Terminal 1: Start server
cargo run --example simple_echo server

# Terminal 2: Start client
cargo run --example simple_echo client
```

## Design Philosophy

This library is **I/O-agnostic by design**. The core protocol (`checksum`, `header`, `packet`, `codec`, `framing`) has zero I/O dependencies. This means:

- Works with sync I/O (`std::io`)
- Works with async I/O (`tokio`, `async-std`)
- Works with in-memory buffers
- Works in `no_std` environments (with `alloc`)
- Easily testable (no mocks needed)

The optional `reader` and `writer` modules provide convenient wrappers for `std::io::Read` and `std::io::Write`, but you can easily create your own adapters for other I/O models.

## Architecture Diagram

```
┌─────────────────────────────────────┐
│   Application Layer                 │
│   (chat, game, RPC, etc.)           │
└─────────────────────────────────────┘
                 ↕
┌─────────────────────────────────────┐
│   I/O Layer (optional)              │
│   PacketReader / PacketWriter       │
│   (std::io adapters)                │
└─────────────────────────────────────┘
                 ↕
┌─────────────────────────────────────┐
│   Protocol Layer (core)             │
│   • FrameDecoder (state machine)    │
│   • encode / decode (codec)         │
│   • Header serialization            │
│   • FNV-1a checksum                 │
└─────────────────────────────────────┘
```

## Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test reads_multiple_packets
```

All tests pass (20 tests total):
- Checksum validation
- Header round-trips
- Packet encoding/decoding
- Codec error handling
- Frame decoder with partial reads
- Frame decoder with multiple packets
- Reader/Writer I/O helpers

## Future Extensions

Possible enhancements (not implemented):

- `no_std` support with feature flag
- Async I/O adapters (Tokio/async-std)
- Compression (zlib, lz4)
- Encryption (optional layer)
- More packet types
- Protocol versioning
- Custom error recovery strategies

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

This means:
- You can use, modify, and distribute this code
- You must share your modifications under the same license
- **If you run this code on a server/service, you must make the source available to users**
- Commercial use is allowed, but modified versions must remain open source

See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions and suggestions are welcome!

## Acknowledgments

Built as an example of protocol design in Rust, demonstrating:
- Binary framing and state machines
- Type-safe packet handling
- Error recovery and resynchronization
- Clean separation of concerns
- Comprehensive testing
