use core::fmt;
use std::error::Error;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Longest accepted install identifier.
const MAXIMUM_INSTALL_ID_BYTES: usize = 128;

/// A stable, random per-installation identifier.
///
/// Randomly generated once and persisted. It is deliberately **not** derived
/// from hardware, user identity, or anything else externally meaningful:
/// rollout only needs a value that is stable and evenly distributed, and
/// anything with outside meaning would turn a staging mechanism into a
/// tracking one.
///
/// Generating the value is the persistence layer's job; this crate is pure.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct InstallId(String);

impl From<InstallId> for String {
    fn from(value: InstallId) -> Self {
        value.0
    }
}

impl InstallId {
    /// Validates and records an install identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, InstallIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(InstallIdError::Empty);
        }
        if value.len() > MAXIMUM_INSTALL_ID_BYTES {
            return Err(InstallIdError::TooLong {
                maximum: MAXIMUM_INSTALL_ID_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for InstallId {
    type Error = InstallIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Install identifier validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallIdError {
    /// The identifier was empty.
    Empty,
    /// The identifier exceeded its bound.
    TooLong {
        /// Accepted maximum.
        maximum: usize,
        /// Supplied length.
        actual: usize,
    },
}

impl fmt::Display for InstallIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("install id must not be empty"),
            Self::TooLong { maximum, actual } => {
                write!(
                    formatter,
                    "install id is {actual} bytes; maximum is {maximum}"
                )
            }
        }
    }
}

impl Error for InstallIdError {}

/// The share of installs a release is offered to.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct RolloutFraction(f64);

impl From<RolloutFraction> for f64 {
    fn from(value: RolloutFraction) -> Self {
        value.0
    }
}

impl RolloutFraction {
    /// Offered to nobody.
    pub const NONE: Self = Self(0.0);
    /// Offered to everybody.
    pub const FULL: Self = Self(1.0);

    /// Validates and records a fraction in `0.0..=1.0`.
    pub fn new(value: f64) -> Result<Self, RolloutFractionError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(RolloutFractionError::OutOfRange { value });
        }
        Ok(Self(value))
    }

    /// Returns the fraction.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for RolloutFraction {
    type Error = RolloutFractionError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Rollout fraction validation failure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RolloutFractionError {
    /// The value was not a finite number in `0.0..=1.0`.
    OutOfRange {
        /// Supplied value.
        value: f64,
    },
}

impl fmt::Display for RolloutFractionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { value } => {
                write!(
                    formatter,
                    "rollout fraction {value} is not within 0.0..=1.0"
                )
            }
        }
    }
}

impl Error for RolloutFractionError {}

/// A staged rollout, as published in a channel manifest.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rollout {
    /// Share of installs currently offered the release.
    pub fraction: RolloutFraction,
    /// Value that fixes each install's position for this release.
    ///
    /// Normally the release version. Changing it reshuffles every install,
    /// which is occasionally what an operator wants after a withdrawn
    /// release and is otherwise a mistake.
    pub seed: String,
}

impl Rollout {
    /// Records a rollout.
    #[must_use]
    pub fn new(fraction: RolloutFraction, seed: impl Into<String>) -> Self {
        Self {
            fraction,
            seed: seed.into(),
        }
    }

    /// Returns whether this install falls inside the current fraction.
    ///
    /// The install's position is a fixed function of its identifier and the
    /// seed, so a given install always lands in the same place for a given
    /// release. Widening the fraction therefore only ever adds installs: an
    /// offer already made is never withdrawn by a later widening.
    #[must_use]
    pub fn includes(&self, install: &InstallId) -> bool {
        // `<` rather than `<=`: at fraction 0.0 nobody is included, and
        // position 0.0 is attainable.
        position(install, &self.seed) < self.fraction.get()
    }
}

/// Maps an install and seed onto a stable position in `0.0..1.0`.
fn position(install: &InstallId, seed: &str) -> f64 {
    let mut hasher = Sha256::new();
    hasher.update(install.as_str().as_bytes());
    // A separator keeps ("ab", "c") from colliding with ("a", "bc").
    hasher.update([0]);
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();

    let mut leading = [0_u8; 8];
    leading.copy_from_slice(&digest[..8]);
    // Divide by 2^64 rather than u64::MAX so the result cannot reach 1.0.
    u64::from_be_bytes(leading) as f64 / (2.0_f64).powi(64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(value: &str) -> InstallId {
        InstallId::new(value).unwrap()
    }

    fn fraction(value: f64) -> RolloutFraction {
        RolloutFraction::new(value).unwrap()
    }

    #[test]
    fn positions_are_within_the_unit_interval() {
        for index in 0..500 {
            let position = position(&install(&format!("install-{index}")), "1.3.0");
            assert!((0.0..1.0).contains(&position), "position {position}");
        }
    }

    #[test]
    fn eligibility_is_stable_across_repeated_checks() {
        let rollout = Rollout::new(fraction(0.5), "1.3.0");
        let install = install("stable-install");
        let first = rollout.includes(&install);

        for _ in 0..100 {
            assert_eq!(rollout.includes(&install), first);
        }
    }

    #[test]
    fn widening_a_rollout_never_withdraws_an_offer() {
        let installs: Vec<_> = (0..300)
            .map(|index| install(&format!("install-{index}")))
            .collect();
        let mut previously_included = vec![false; installs.len()];

        for step in 0..=20 {
            let rollout = Rollout::new(fraction(f64::from(step) / 20.0), "1.3.0");
            for (index, install) in installs.iter().enumerate() {
                let included = rollout.includes(install);
                assert!(
                    !previously_included[index] || included,
                    "widening withdrew an offer already made to install {index}"
                );
                previously_included[index] = included;
            }
        }

        assert!(
            previously_included.iter().all(|included| *included),
            "a full rollout must include every install"
        );
    }

    #[test]
    fn a_zero_fraction_includes_nobody_and_a_full_fraction_includes_everybody() {
        let none = Rollout::new(RolloutFraction::NONE, "1.3.0");
        let full = Rollout::new(RolloutFraction::FULL, "1.3.0");

        for index in 0..200 {
            let install = install(&format!("install-{index}"));
            assert!(!none.includes(&install));
            assert!(full.includes(&install));
        }
    }

    #[test]
    fn a_half_rollout_lands_near_half_the_installs() {
        let rollout = Rollout::new(fraction(0.5), "1.3.0");
        let included = (0..2_000)
            .filter(|index| rollout.includes(&install(&format!("install-{index}"))))
            .count();

        assert!(
            (900..=1_100).contains(&included),
            "expected roughly half of 2000 installs, found {included}"
        );
    }

    #[test]
    fn changing_the_seed_reshuffles_positions() {
        let installs: Vec<_> = (0..200)
            .map(|index| install(&format!("install-{index}")))
            .collect();
        let first = Rollout::new(fraction(0.5), "1.3.0");
        let second = Rollout::new(fraction(0.5), "1.3.1");

        let moved = installs
            .iter()
            .filter(|install| first.includes(install) != second.includes(install))
            .count();

        assert!(
            moved > 20,
            "a new seed should reshuffle, only {moved} moved"
        );
    }

    #[test]
    fn the_separator_keeps_adjacent_splits_apart() {
        assert_ne!(position(&install("ab"), "c"), position(&install("a"), "bc"));
    }

    #[test]
    fn fractions_outside_the_unit_interval_are_refused() {
        assert!(RolloutFraction::new(-0.1).is_err());
        assert!(RolloutFraction::new(1.1).is_err());
        assert!(RolloutFraction::new(f64::NAN).is_err());
        assert!(RolloutFraction::new(f64::INFINITY).is_err());
    }

    #[test]
    fn install_ids_are_bounded_and_non_empty() {
        assert_eq!(InstallId::new(""), Err(InstallIdError::Empty));
        assert!(matches!(
            InstallId::new("x".repeat(MAXIMUM_INSTALL_ID_BYTES + 1)),
            Err(InstallIdError::TooLong { .. })
        ));
        assert!(InstallId::new("x".repeat(MAXIMUM_INSTALL_ID_BYTES)).is_ok());
    }
}
