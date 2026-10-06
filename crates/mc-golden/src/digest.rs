//! blake3 digests over canonical byte arrays (ADR-007 §5.2 item 2).

/// Lower-case hex blake3 of `bytes` (64 characters).
///
/// ```
/// assert_eq!(
///     mc_golden::blake3_hex(b""),
///     "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
/// );
/// ```
pub fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// True when `s` looks like a [`blake3_hex`] result: 64 lower-case hex digits.
pub fn is_blake3_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let d = blake3_hex(&[]);
        assert_eq!(
            d,
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
        assert!(is_blake3_hex(&d));
        assert!(!is_blake3_hex(&d.to_uppercase()));
        assert!(!is_blake3_hex(&d[1..]));
    }
}
