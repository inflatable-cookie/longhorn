use std::{error::Error, fmt};

/// Nonzero epoch for one live operation authority instance.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationAuthorityEpoch(u64);

impl OperationAuthorityEpoch {
    /// Validates and constructs an authority epoch.
    pub const fn new(value: u64) -> Result<Self, OperationAuthorityEpochError> {
        if value == 0 {
            Err(OperationAuthorityEpochError)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Authority epoch zero is reserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationAuthorityEpochError;

impl fmt::Display for OperationAuthorityEpochError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operation authority epoch must be nonzero")
    }
}

impl Error for OperationAuthorityEpochError {}

/// Monotonic insertion sequence for one operation catalogue.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationSequence(u64);

impl OperationSequence {
    /// First sequence allocated by an empty catalogue.
    pub const FIRST: Self = Self(1);

    /// Constructs a nonzero sequence.
    pub const fn new(value: u64) -> Result<Self, OperationSequenceZero> {
        if value == 0 {
            Err(OperationSequenceZero)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next sequence or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, OperationSequenceOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(OperationSequenceOverflow),
        }
    }
}

/// Operation sequence zero is reserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationSequenceZero;

impl fmt::Display for OperationSequenceZero {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operation sequence must be nonzero")
    }
}

impl Error for OperationSequenceZero {}

/// An operation sequence could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationSequenceOverflow;

impl fmt::Display for OperationSequenceOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operation sequence cannot advance beyond u64::MAX")
    }
}

impl Error for OperationSequenceOverflow {}

/// Monotonic progress sequence for one operation.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationProgressSequence(u64);

impl OperationProgressSequence {
    /// Initial sequence before any progress update is committed.
    pub const INITIAL: Self = Self(0);

    /// Constructs a progress sequence from its protocol value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next sequence or fails instead of wrapping.
    pub const fn checked_next(self) -> Result<Self, OperationProgressSequenceOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(OperationProgressSequenceOverflow),
        }
    }
}

/// An operation progress sequence could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationProgressSequenceOverflow;

impl fmt::Display for OperationProgressSequenceOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operation progress sequence cannot advance beyond u64::MAX")
    }
}

impl Error for OperationProgressSequenceOverflow {}
