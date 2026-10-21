use std::{error::Error, fmt};

/// Lowercase hexadecimal SHA-256 digest of one resolved storage layout.
///
/// The digest binds profile, platform, identity, leaf provenance, every
/// resolved root with its provenance, and the profile warnings. It is layout
/// evidence for the fixed locator, not a payload hash.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StorageLayoutDigest(String);

impl StorageLayoutDigest {
    /// Wraps an already finalized SHA-256 output.
    pub(crate) fn from_output(output: &[u8]) -> Self {
        Self(bytes_to_lowercase_hex(output))
    }

    /// Validates an existing lowercase hexadecimal digest.
    pub fn new(value: impl Into<String>) -> Result<Self, StorageLayoutDigestError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(StorageLayoutDigestError);
        }
        Ok(Self(value))
    }

    /// Returns the lowercase hexadecimal digest.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Invalid serialized SHA-256 digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageLayoutDigestError;

impl fmt::Display for StorageLayoutDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SHA-256 digest must be 64 lowercase hexadecimal characters")
    }
}

impl Error for StorageLayoutDigestError {}

pub(crate) fn bytes_to_lowercase_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push(char::from_digit(u32::from(byte >> 4), 16).expect("high nibble is a hex digit"));
        hex.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("low nibble is a hex digit"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn digest_output_is_lowercase_hex_and_validated_round_trip() {
        let digest = StorageLayoutDigest::from_output(&Sha256::digest(b"layout"));
        assert_eq!(digest.as_str().len(), 64);
        assert_eq!(StorageLayoutDigest::new(digest.as_str()).unwrap(), digest);
        assert_eq!(
            StorageLayoutDigest::new("NOT-HEX").unwrap_err(),
            StorageLayoutDigestError
        );
    }
}
