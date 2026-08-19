//! Per-instance bearer token: the entire trust boundary of the control
//! surface (contract 022).
//!
//! `evaluate` and command invocation are full code execution in the app, so
//! the token is treated as a credential throughout: generated from the OS
//! CSPRNG, compared in constant time, redacted in `Debug`, and never logged.

use std::{error::Error, fmt};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// Random bytes behind one token, drawn from the OS CSPRNG.
const TOKEN_RANDOM_BYTES: usize = 32;

/// Exact length of one encoded token: 32 bytes, base64url without padding.
const TOKEN_CHARS: usize = 43;

/// Token generation or construction failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenError {
    /// The operating system CSPRNG was unavailable.
    EntropyUnavailable,
    /// Token was not exactly 43 characters.
    InvalidLength {
        /// Length actually supplied.
        actual: usize,
    },
    /// Token carried a symbol outside the base64url alphabet.
    InvalidSymbol {
        /// The offending symbol.
        symbol: char,
    },
}

impl fmt::Display for TokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntropyUnavailable => formatter.write_str("operating system CSPRNG unavailable"),
            Self::InvalidLength { actual } => {
                write!(
                    formatter,
                    "token must be {TOKEN_CHARS} characters, got {actual}"
                )
            }
            Self::InvalidSymbol { symbol } => {
                write!(
                    formatter,
                    "token symbol {symbol:?} is outside the base64url alphabet"
                )
            }
        }
    }
}

impl Error for TokenError {}

/// A per-instance bearer token.
///
/// Held as a `SecretString`: zeroized on drop and redacted in `Debug`, the
/// same posture `longhorn-licence` gives account credentials. The one place
/// the plaintext legitimately travels is the discovery file, whose directory
/// is agent-readable by design — that file is how an agent learns the token.
#[derive(Clone, Debug)]
pub struct InstanceToken(SecretString);

impl PartialEq for InstanceToken {
    /// Secret comparison; same constant-time posture as licence verifiers.
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(
            self.0.expose_secret().as_bytes(),
            other.0.expose_secret().as_bytes(),
        )
    }
}

impl Eq for InstanceToken {}

impl InstanceToken {
    /// Generates a fresh token from the operating system's CSPRNG.
    pub fn generate() -> Result<Self, TokenError> {
        let mut bytes = [0_u8; TOKEN_RANDOM_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| TokenError::EntropyUnavailable)?;
        Self::new(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Validates and records a token, e.g. one read back from a discovery
    /// file. A weak token cannot exist: the length and alphabet rules are
    /// enforced either way.
    pub fn new(value: impl Into<String>) -> Result<Self, TokenError> {
        let value = value.into();
        if value.len() != TOKEN_CHARS {
            return Err(TokenError::InvalidLength {
                actual: value.len(),
            });
        }
        if let Some(symbol) = value.chars().find(|symbol| !is_base64url(*symbol)) {
            return Err(TokenError::InvalidSymbol { symbol });
        }
        Ok(Self(SecretString::from(value)))
    }

    /// Returns the token plaintext. Legitimate callers are the discovery
    /// file writer and the `Authorization` header builder; nothing else
    /// should need this.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.expose_secret()
    }

    /// Constant-time check of a presented bearer value against this token.
    #[must_use]
    pub fn verify(&self, presented: &str) -> bool {
        constant_time_eq(self.0.expose_secret().as_bytes(), presented.as_bytes())
    }
}

impl Serialize for InstanceToken {
    /// The discovery file carries the token by design (contract 022); every
    /// other serializer consumer should treat that as a credential leak.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InstanceToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Length-sensitive-prefix-free byte equality: mismatched lengths still fold
/// over the shared prefix, and the length difference alone decides.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for (left, right) in left.iter().zip(right.iter()) {
        difference |= usize::from(left ^ right);
    }
    difference == 0
}

/// RFC 4648 base64url alphabet without padding.
const fn is_base64url(symbol: char) -> bool {
    symbol.is_ascii_alphanumeric() || matches!(symbol, '-' | '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_validate_and_round_trip() {
        let token = InstanceToken::generate().unwrap();
        assert_eq!(token.as_str().len(), TOKEN_CHARS);
        InstanceToken::new(token.as_str()).unwrap();

        let other = InstanceToken::generate().unwrap();
        assert!(token.verify(token.as_str()));
        assert!(!token.verify(other.as_str()));
        assert_ne!(token, other);
    }

    #[test]
    fn weak_tokens_cannot_exist() {
        assert_eq!(
            InstanceToken::new("short").unwrap_err(),
            TokenError::InvalidLength { actual: 5 }
        );
        let bad = format!("{}!", "a".repeat(TOKEN_CHARS - 1));
        assert!(matches!(
            InstanceToken::new(bad).unwrap_err(),
            TokenError::InvalidSymbol { symbol: '!' }
        ));
    }

    #[test]
    fn verify_is_constant_time_shaped() {
        let token = InstanceToken::generate().unwrap();
        let mut wrong = token.as_str().to_owned();
        wrong.replace_range(..1, if wrong.starts_with('a') { "b" } else { "a" });
        assert!(!token.verify(&wrong));
        assert!(!token.verify(""));
        assert!(!token.verify(&format!("{}x", token.as_str())));
    }
}
