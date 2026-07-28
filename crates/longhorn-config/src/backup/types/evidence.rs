use std::{
    error::Error,
    fmt,
    io::{self, Read},
};

use longhorn_core::DomainId;
use serde::{Deserialize, Deserializer, Serialize, de};
use sha2::{Digest, Sha256};

use super::identity::MAX_METADATA_BYTES;

/// Lowercase hexadecimal SHA-256 digest.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// Computes a digest from exact bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }

    /// Validates an existing lowercase hexadecimal digest.
    pub fn new(value: impl Into<String>) -> Result<Self, Sha256DigestError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(Sha256DigestError);
        }
        Ok(Self(value))
    }

    /// Returns the lowercase hexadecimal digest.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn from_reader(mut reader: impl Read) -> io::Result<(u64, Self)> {
        let mut digest = Sha256::new();
        let mut length = 0_u64;
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            length = length
                .checked_add(read as u64)
                .ok_or_else(|| io::Error::other("source length exceeds u64"))?;
            digest.update(&buffer[..read]);
        }
        Ok((length, Self(format!("{:x}", digest.finalize()))))
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Invalid serialized SHA-256 digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha256DigestError;

impl fmt::Display for Sha256DigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SHA-256 digest must be 64 lowercase hexadecimal characters")
    }
}

impl Error for Sha256DigestError {}

/// Validated payload path recorded before archive encoding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BackupPayloadPath(String);

impl BackupPayloadPath {
    pub(crate) fn ordinary(domain: &DomainId) -> Self {
        Self(format!("longhorn/domains/{domain}.json"))
    }

    pub(crate) fn adapter(
        domain: &DomainId,
        relative: &crate::backup::BackupAdapterRelativePath,
    ) -> Self {
        Self(format!("longhorn/adapters/{domain}/{}", relative.as_str()))
    }

    /// Validates an archive-relative payload path.
    pub fn new(value: impl Into<String>) -> Result<Self, BackupPayloadPathError> {
        let value = value.into();
        if !value.starts_with("longhorn/") {
            return Err(BackupPayloadPathError::OutsideNamespace);
        }
        if value.len() > MAX_METADATA_BYTES {
            return Err(BackupPayloadPathError::TooLong);
        }
        if value.contains('\\') || value.contains('\0') {
            return Err(BackupPayloadPathError::InvalidCharacter);
        }
        if value.split('/').any(|segment| {
            segment.is_empty()
                || matches!(segment, "." | "..")
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        }) {
            return Err(BackupPayloadPathError::InvalidSegment);
        }
        Ok(Self(value))
    }

    /// Returns the portable archive-relative payload path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for BackupPayloadPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Invalid portable payload path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupPayloadPathError {
    /// Path does not begin below the Longhorn namespace.
    OutsideNamespace,
    /// Path exceeds the bounded manifest text limit.
    TooLong,
    /// Path contains a backslash or NUL.
    InvalidCharacter,
    /// Path contains an empty, traversal, or non-portable segment.
    InvalidSegment,
}

impl fmt::Display for BackupPayloadPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideNamespace => {
                formatter.write_str("backup payload path must begin with longhorn/")
            }
            Self::TooLong => formatter.write_str("backup payload path is too long"),
            Self::InvalidCharacter => {
                formatter.write_str("backup payload path contains a forbidden character")
            }
            Self::InvalidSegment => {
                formatter.write_str("backup payload path contains a non-portable segment")
            }
        }
    }
}

impl Error for BackupPayloadPathError {}

/// Exact payload evidence recorded in a manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupPayloadManifest {
    path: BackupPayloadPath,
    byte_length: u64,
    sha256: Sha256Digest,
}

impl BackupPayloadManifest {
    pub(crate) fn new(path: BackupPayloadPath, bytes: &[u8]) -> Self {
        Self {
            path,
            byte_length: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(bytes),
        }
    }

    /// Returns the portable payload path.
    #[must_use]
    pub fn path(&self) -> &BackupPayloadPath {
        &self.path
    }

    /// Returns the exact payload byte length.
    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Returns the exact payload digest.
    #[must_use]
    pub fn sha256(&self) -> &Sha256Digest {
        &self.sha256
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_strict_lowercase_sha256() {
        let digest = Sha256Digest::from_bytes(b"abc");
        assert_eq!(
            digest.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(Sha256Digest::new(digest.as_str()).unwrap(), digest);
        assert!(Sha256Digest::new(digest.as_str().to_uppercase()).is_err());
        assert!(Sha256Digest::new("short").is_err());
    }

    #[test]
    fn payload_paths_are_portable_and_namespace_confined() {
        assert!(BackupPayloadPath::new("longhorn/domains/example.json").is_ok());
        assert!(BackupPayloadPath::new("../example.json").is_err());
        assert!(BackupPayloadPath::new("longhorn/../example.json").is_err());
        assert!(BackupPayloadPath::new("longhorn\\domains\\example.json").is_err());
        assert!(BackupPayloadPath::new("longhorn/domains/example name.json").is_err());
    }
}
