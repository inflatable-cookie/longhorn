use core::fmt;
use std::error::Error;

use serde::{Deserialize, Serialize};

/// Crockford base32: no I, L, O, or U.
///
/// I/L/O are excluded because they are indistinguishable from 1 and 0 in most
/// typefaces and in handwriting. U is excluded because omitting it makes
/// accidental profanity in generated keys far less likely.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Crockford's check alphabet: the 32 symbols plus five check-only symbols,
/// giving the prime modulus 37.
const CHECK_ALPHABET: &[u8; 37] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ*~$=U";

/// Symbols per group in the display form.
const GROUP: usize = 5;

/// The shortest key that may exist, check symbol included.
///
/// Not a typing convenience: it is what makes `NotRecognised` safe to
/// distinguish from `Revoked`. Telling a caller that a well-formed key is not
/// recognised lets them learn which keys exist, and that only matters if keys
/// are guessable. Crockford base32 is five bits a symbol, so nineteen body
/// symbols is ninety-five bits and enumeration is not a threat.
///
/// Twenty also renders as four clean groups of five, which is the form
/// `grouped` already prints.
pub const MINIMUM_KEY_SYMBOLS: usize = 20;

/// A licence key as typed by a customer.
///
/// A **redemption token**, never a self-verifying artifact. An Ed25519
/// signature is 103 base32 characters, which nobody types, and the schemes
/// that squeeze verification into a short key are keygen-able permanently
/// the moment one person reverses them. So this key identifies an order; the
/// authority is the licence it redeems for.
///
/// The check character catches typing mistakes locally. It is **not** a
/// security feature and does not need to be: forging a key that passes it
/// still fails redemption, and the point is to say "you mistyped that"
/// instead of round-tripping to a server that replies "invalid key" and
/// leaves the customer believing they were sold a dud.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct LicenceKey(String);

impl From<LicenceKey> for String {
    fn from(value: LicenceKey) -> Self {
        value.0
    }
}

impl LicenceKey {
    /// Parses a key as typed, accepting anything a person plausibly enters.
    ///
    /// Lower case, missing dashes, extra dashes, and surrounding or embedded
    /// whitespace are all accepted. So are the classic confusions: `I` and
    /// `L` read as `1`, `O` reads as `0`. Rejecting these would be rejecting
    /// the customer for the typeface's mistake.
    pub fn parse(input: &str) -> Result<Self, LicenceKeyError> {
        let normalized = normalize(input)?;
        if normalized.len() < MINIMUM_KEY_SYMBOLS {
            return Err(LicenceKeyError::TooShort {
                minimum: MINIMUM_KEY_SYMBOLS,
                actual: normalized.len(),
            });
        }

        let (body, check) = normalized.split_at(normalized.len() - 1);
        let expected = check_symbol(body);
        let actual = check.as_bytes()[0];
        if actual != expected {
            return Err(LicenceKeyError::CheckFailed);
        }

        Ok(Self(normalized))
    }

    /// Appends the correct check symbol to a body, producing a valid key.
    ///
    /// The issuing side of the same rule, kept here so the two cannot drift.
    pub fn from_body(body: &str) -> Result<Self, LicenceKeyError> {
        let normalized = normalize(body)?;
        // The body is one symbol shorter than the key it becomes.
        if normalized.len() + 1 < MINIMUM_KEY_SYMBOLS {
            return Err(LicenceKeyError::TooShort {
                minimum: MINIMUM_KEY_SYMBOLS,
                actual: normalized.len() + 1,
            });
        }
        let check = check_symbol(&normalized);
        Ok(Self(format!("{normalized}{}", char::from(check))))
    }

    /// Returns the normalized key, without grouping.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the key grouped for display, as `XXXXX-XXXXX-...`.
    #[must_use]
    pub fn grouped(&self) -> String {
        self.0
            .as_bytes()
            .chunks(GROUP)
            .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
            .collect::<Vec<_>>()
            .join("-")
    }
}

impl TryFrom<String> for LicenceKey {
    type Error = LicenceKeyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl fmt::Display for LicenceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.grouped())
    }
}

/// Normalizes typed input into canonical symbols.
fn normalize(input: &str) -> Result<String, LicenceKeyError> {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        if character.is_whitespace() || character == '-' {
            continue;
        }
        let upper = character.to_ascii_uppercase();
        let mapped = match upper {
            // The confusions Crockford excluded the symbols to avoid.
            'I' | 'L' => '1',
            'O' => '0',
            other => other,
        };
        if !mapped.is_ascii() || !CHECK_ALPHABET.contains(&(mapped as u8)) {
            return Err(LicenceKeyError::UnexpectedSymbol { symbol: character });
        }
        output.push(mapped);
    }
    Ok(output)
}

/// Computes the check symbol for a normalized body.
///
/// A position-weighted sum modulo 37. The weighting is what catches
/// transpositions; an unweighted sum would accept any reordering of the same
/// symbols, which is one of the two mistakes people actually make.
fn check_symbol(body: &str) -> u8 {
    let sum: usize = body
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(index, symbol)| {
            let value = ALPHABET
                .iter()
                .position(|candidate| candidate == symbol)
                .unwrap_or(0);
            value * (index + 1)
        })
        .sum();
    CHECK_ALPHABET[sum % 37]
}

/// Licence key parsing failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LicenceKeyError {
    /// A symbol was not part of the alphabet.
    UnexpectedSymbol {
        /// The offending character, as typed.
        symbol: char,
    },
    /// There were not enough symbols to be a key.
    TooShort {
        /// Minimum symbol count.
        minimum: usize,
        /// Supplied symbol count.
        actual: usize,
    },
    /// The check symbol did not match the body.
    CheckFailed,
}

impl LicenceKeyError {
    /// Returns whether this failure means the key was mistyped.
    ///
    /// Distinguishing a typo from a malformed entry is what lets a surface
    /// say "check that key" rather than implying the key is worthless.
    #[must_use]
    pub const fn is_probably_a_typo(self) -> bool {
        matches!(self, Self::CheckFailed | Self::UnexpectedSymbol { .. })
    }
}

impl fmt::Display for LicenceKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedSymbol { symbol } => {
                write!(formatter, "`{symbol}` is not part of a licence key")
            }
            Self::TooShort { minimum, actual } => write!(
                formatter,
                "licence key has {actual} symbols; at least {minimum} are needed"
            ),
            Self::CheckFailed => formatter.write_str("licence key looks mistyped"),
        }
    }
}

impl Error for LicenceKeyError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nineteen body symbols, so the issued key is the twenty the entropy
    /// floor requires.
    fn key() -> LicenceKey {
        LicenceKey::from_body("ABCDE12345FGHJK6789").unwrap()
    }

    #[test]
    fn an_issued_key_parses_back() {
        let issued = key();

        assert_eq!(LicenceKey::parse(issued.as_str()).unwrap(), issued);
    }

    #[test]
    fn keys_display_in_groups_of_five() {
        let issued = key();
        let check = &issued.as_str()[19..];

        assert_eq!(issued.grouped(), format!("ABCDE-12345-FGHJK-6789{check}"));
    }

    #[test]
    fn typed_variations_all_parse_to_the_same_key() {
        let issued = key();

        let plain = issued.as_str().to_owned();
        let grouped = issued.grouped();
        for typed in [
            plain.clone(),
            plain.to_lowercase(),
            grouped.clone(),
            format!("  {}  ", grouped.replace('-', " ")),
            grouped.to_lowercase(),
        ] {
            let typed = typed.as_str();
            assert_eq!(
                LicenceKey::parse(typed).unwrap(),
                issued,
                "{typed} should parse"
            );
        }
    }

    #[test]
    fn the_classic_confusions_are_accepted_not_rejected() {
        // A customer reading `1` as `I` has made the typeface's mistake, not
        // their own. `0` and `O` likewise.
        let issued = LicenceKey::from_body("1010123456789ABCDEF").unwrap();

        let check = &issued.as_str()[5..];

        assert_eq!(LicenceKey::parse(&format!("IOIOI{check}")).unwrap(), issued);
        assert_eq!(LicenceKey::parse(&format!("LOLOL{check}")).unwrap(), issued);
    }

    #[test]
    fn a_single_wrong_symbol_is_caught() {
        let issued = key();
        let mut wrong = issued.as_str().to_owned();
        wrong.replace_range(0..1, "B");

        assert_eq!(LicenceKey::parse(&wrong), Err(LicenceKeyError::CheckFailed));
    }

    #[test]
    fn a_transposition_is_caught() {
        // The reason the checksum is position-weighted. An unweighted sum
        // accepts any reordering, and transposition is one of the two
        // mistakes people actually make.
        let issued = key();
        let mut swapped = issued.as_str().to_owned();
        swapped.replace_range(0..2, "BA");

        assert_eq!(
            LicenceKey::parse(&swapped),
            Err(LicenceKeyError::CheckFailed)
        );
    }

    #[test]
    fn a_symbol_outside_the_alphabet_is_reported_as_typed() {
        assert_eq!(
            LicenceKey::parse("ABCDE!2345FGHJK6789"),
            Err(LicenceKeyError::UnexpectedSymbol { symbol: '!' })
        );
    }

    #[test]
    fn both_failure_modes_read_as_typos_to_a_surface() {
        assert!(LicenceKeyError::CheckFailed.is_probably_a_typo());
        assert!(LicenceKeyError::UnexpectedSymbol { symbol: '!' }.is_probably_a_typo());
        assert!(
            !LicenceKeyError::TooShort {
                minimum: MINIMUM_KEY_SYMBOLS,
                actual: 0
            }
            .is_probably_a_typo()
        );
    }

    #[test]
    fn an_empty_key_is_not_a_typo_report() {
        assert!(matches!(
            LicenceKey::parse(""),
            Err(LicenceKeyError::TooShort { .. })
        ));
    }

    /// The entropy floor. Distinguishing "not recognised" from "revoked" is
    /// only safe while a well-formed key cannot be enumerated, and nothing
    /// else in the type stops a five-bit key being minted.
    #[test]
    fn a_body_too_short_to_be_unguessable_cannot_be_issued() {
        assert_eq!(
            LicenceKey::from_body("ABCDE"),
            Err(LicenceKeyError::TooShort {
                minimum: MINIMUM_KEY_SYMBOLS,
                actual: 6,
            })
        );
    }

    /// A short key reports its length, not its shape. A mistyped key must
    /// never read as an invalid one, and the inverse holds too.
    #[test]
    fn a_short_key_says_short_rather_than_malformed() {
        let error = LicenceKey::parse("ABCDE12345").unwrap_err();

        assert!(matches!(error, LicenceKeyError::TooShort { .. }));
        assert!(!error.is_probably_a_typo());
        assert!(error.to_string().contains("at least 20"));
    }
}
