//! Lowercase hexadecimal byte encoding.

/// Encodes bytes as lowercase hexadecimal (`{byte:02x}` per byte).
#[must_use]
pub fn bytes_to_lowercase_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::bytes_to_lowercase_hex;

    #[test]
    fn encodes_empty_input() {
        assert_eq!(bytes_to_lowercase_hex(&[]), "");
    }

    #[test]
    fn encodes_lowercase_hex() {
        assert_eq!(bytes_to_lowercase_hex(&[0x00, 0x0a, 0xff]), "000aff");
    }
}
