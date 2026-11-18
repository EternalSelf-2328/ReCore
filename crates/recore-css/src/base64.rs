//! Base64, for the fonts and pictures themes carry in `data:` URIs. No
//! dependency, and forgiving about the whitespace editors add.

/// Decodes standard or URL-safe base64, with or without padding.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4 + 3);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => u32::from(byte - b'A'),
            b'a'..=b'z' => u32::from(byte - b'a') + 26,
            b'0'..=b'9' => u32::from(byte - b'0') + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b'\t' | b' ' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_padded_and_unpadded() {
        assert_eq!(decode("AAEC"), Some(vec![0, 1, 2]));
        assert_eq!(decode("AAE="), Some(vec![0, 1]));
        assert_eq!(decode("AAE"), Some(vec![0, 1]));
        assert_eq!(decode("aGVsbG8="), Some(b"hello".to_vec()));
    }

    #[test]
    fn rejects_what_is_not_base64() {
        assert_eq!(decode("hello!"), None);
    }
}
