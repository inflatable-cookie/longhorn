use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Monotonic revision of one durable layout document.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct LayoutRevision(u64);

impl LayoutRevision {
    /// Initial revision for a new layout document.
    pub const INITIAL: Self = Self(0);

    /// Constructs a revision from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized revision value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next revision or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, LayoutRevisionOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(LayoutRevisionOverflow),
        }
    }
}

/// A layout revision could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutRevisionOverflow;

impl fmt::Display for LayoutRevisionOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("layout revision cannot advance beyond u64::MAX")
    }
}

impl Error for LayoutRevisionOverflow {}

/// Monotonic revision of one durable Surface document.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct SurfaceRevision(u64);

impl SurfaceRevision {
    /// Initial revision for a new Surface document.
    pub const INITIAL: Self = Self(0);

    /// Constructs a revision from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized revision value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next revision or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, SurfaceRevisionOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(SurfaceRevisionOverflow),
        }
    }
}

/// A Surface revision could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceRevisionOverflow;

impl fmt::Display for SurfaceRevisionOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Surface revision cannot advance beyond u64::MAX")
    }
}

impl Error for SurfaceRevisionOverflow {}

/// Monotonic structural revision of one history authority.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct HistoryRevision(u64);

impl HistoryRevision {
    /// Initial revision for an empty history authority.
    pub const INITIAL: Self = Self(0);

    /// Constructs a revision from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized revision value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next revision or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, HistoryRevisionOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(HistoryRevisionOverflow),
        }
    }
}

/// A history revision could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryRevisionOverflow;

impl fmt::Display for HistoryRevisionOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("history revision cannot advance beyond u64::MAX")
    }
}

impl Error for HistoryRevisionOverflow {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_is_monotonic_and_never_wraps() {
        assert_eq!(LayoutRevision::INITIAL.checked_next().unwrap().get(), 1);
        assert_eq!(
            LayoutRevision::new(u64::MAX).checked_next(),
            Err(LayoutRevisionOverflow)
        );
        assert_eq!(SurfaceRevision::INITIAL.checked_next().unwrap().get(), 1);
        assert_eq!(
            SurfaceRevision::new(u64::MAX).checked_next(),
            Err(SurfaceRevisionOverflow)
        );
        assert_eq!(HistoryRevision::INITIAL.checked_next().unwrap().get(), 1);
        assert_eq!(
            HistoryRevision::new(u64::MAX).checked_next(),
            Err(HistoryRevisionOverflow)
        );
    }

    #[test]
    fn revision_serializes_as_an_integer() {
        let revision = LayoutRevision::new(42);
        let json = serde_json::to_string(&revision).unwrap();

        assert_eq!(json, "42");
        assert_eq!(
            serde_json::from_str::<LayoutRevision>(&json).unwrap(),
            revision
        );

        let surface_revision = SurfaceRevision::new(73);
        assert_eq!(
            serde_json::from_str::<SurfaceRevision>(
                &serde_json::to_string(&surface_revision).unwrap()
            )
            .unwrap(),
            surface_revision
        );

        let history_revision = HistoryRevision::new(91);
        assert_eq!(
            serde_json::from_str::<HistoryRevision>(
                &serde_json::to_string(&history_revision).unwrap()
            )
            .unwrap(),
            history_revision
        );
    }
}
