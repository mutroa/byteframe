//! Pure Rust implementation of the 32-bit FNV-1a checksum.

pub const FNV_OFFSET_BASIS: u32 = 0x811C9DC5;
pub const FNV_PRIME: u32 = 0x01000193;

pub fn fnv1a32(data: &[u8]) -> u32 {
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::fnv1a32;

    #[test]
    fn matches_known_vectors() {
        assert_eq!(fnv1a32(b""), 0x811C9DC5);
        assert_eq!(fnv1a32(b"a"), 0xE40C292C);
        assert_eq!(fnv1a32(b"hello"), 0x4F9F2CAB);
    }
}
