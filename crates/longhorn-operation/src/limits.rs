use std::{error::Error, fmt};

/// Defensive ceiling for one consumer-owned operation label.
pub const MAXIMUM_OPERATION_LABEL_BYTES: usize = 4_096;
/// Defensive ceiling for one consumer-owned phase label.
pub const MAXIMUM_OPERATION_PHASE_LABEL_BYTES: usize = 4_096;
/// Defensive ceiling for one process-local retained catalogue.
pub const MAXIMUM_RETAINED_OPERATIONS: usize = 65_536;
/// Defensive ceiling for terminal metadata retained by one catalogue.
pub const MAXIMUM_OPERATION_ENCODED_WEIGHT: u64 = 1 << 40;

/// Nonempty, hard-bounded consumer-owned operation label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationLabel(String);

impl OperationLabel {
    /// Validates and constructs an operation label.
    pub fn new(value: impl Into<String>) -> Result<Self, OperationLabelError> {
        let value = value.into();
        if value.is_empty() {
            return Err(OperationLabelError::Empty);
        }
        if value.len() > MAXIMUM_OPERATION_LABEL_BYTES {
            return Err(OperationLabelError::TooLong {
                maximum: MAXIMUM_OPERATION_LABEL_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the consumer-owned presentation label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Invalid operation label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationLabelError {
    /// The label was empty.
    Empty,
    /// The label exceeded the hard byte ceiling.
    TooLong {
        /// Hard byte ceiling.
        maximum: usize,
        /// Supplied byte count.
        actual: usize,
    },
}

impl fmt::Display for OperationLabelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("operation label cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "operation label is {actual} bytes; maximum is {maximum}"
            ),
        }
    }
}

impl Error for OperationLabelError {}

/// Nonempty, hard-bounded consumer-owned phase label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationPhaseLabel(String);

impl OperationPhaseLabel {
    /// Validates and constructs a phase label.
    pub fn new(value: impl Into<String>) -> Result<Self, OperationPhaseLabelError> {
        let value = value.into();
        if value.is_empty() {
            return Err(OperationPhaseLabelError::Empty);
        }
        if value.len() > MAXIMUM_OPERATION_PHASE_LABEL_BYTES {
            return Err(OperationPhaseLabelError::TooLong {
                maximum: MAXIMUM_OPERATION_PHASE_LABEL_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the consumer-owned presentation label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Invalid operation phase label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationPhaseLabelError {
    /// The label was empty.
    Empty,
    /// The label exceeded the hard byte ceiling.
    TooLong {
        /// Hard byte ceiling.
        maximum: usize,
        /// Supplied byte count.
        actual: usize,
    },
}

impl fmt::Display for OperationPhaseLabelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("operation phase label cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "operation phase label is {actual} bytes; maximum is {maximum}"
            ),
        }
    }
}

impl Error for OperationPhaseLabelError {}

/// Explicit finite bound for one operation catalogue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationCatalogueLimits {
    maximum_active_operations: usize,
    maximum_terminal_operations: usize,
    maximum_terminal_encoded_weight: u64,
}

impl OperationCatalogueLimits {
    /// Default process-local catalogue bound.
    pub const DEFAULT: Self = Self {
        maximum_active_operations: 4_096,
        maximum_terminal_operations: 100,
        maximum_terminal_encoded_weight: 16 * 1_024 * 1_024,
    };

    /// Validates and constructs catalogue limits.
    pub const fn new(
        maximum_active_operations: usize,
        maximum_terminal_operations: usize,
        maximum_terminal_encoded_weight: u64,
    ) -> Result<Self, OperationCatalogueLimitsError> {
        if maximum_active_operations == 0 {
            return Err(OperationCatalogueLimitsError::ZeroActiveLimit);
        }
        let total = match maximum_active_operations.checked_add(maximum_terminal_operations) {
            Some(total) => total,
            None => return Err(OperationCatalogueLimitsError::TooManyOperations),
        };
        if total > MAXIMUM_RETAINED_OPERATIONS {
            return Err(OperationCatalogueLimitsError::TooManyOperations);
        }
        if maximum_terminal_encoded_weight > MAXIMUM_OPERATION_ENCODED_WEIGHT {
            return Err(OperationCatalogueLimitsError::EncodedWeightTooLarge {
                maximum: MAXIMUM_OPERATION_ENCODED_WEIGHT,
                actual: maximum_terminal_encoded_weight,
            });
        }
        Ok(Self {
            maximum_active_operations,
            maximum_terminal_operations,
            maximum_terminal_encoded_weight,
        })
    }

    /// Returns the maximum simultaneously active operations.
    #[must_use]
    pub const fn maximum_active_operations(self) -> usize {
        self.maximum_active_operations
    }

    /// Returns the maximum retained terminal operations.
    #[must_use]
    pub const fn maximum_terminal_operations(self) -> usize {
        self.maximum_terminal_operations
    }

    /// Returns the maximum retained terminal metadata weight.
    #[must_use]
    pub const fn maximum_terminal_encoded_weight(self) -> u64 {
        self.maximum_terminal_encoded_weight
    }
}

impl Default for OperationCatalogueLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Invalid operation catalogue limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationCatalogueLimitsError {
    /// A catalogue must admit at least one active operation.
    ZeroActiveLimit,
    /// Active plus terminal count exceeded the defensive ceiling.
    TooManyOperations,
    /// Terminal metadata weight exceeded the defensive ceiling.
    EncodedWeightTooLarge {
        /// Defensive ceiling.
        maximum: u64,
        /// Supplied weight.
        actual: u64,
    },
}

impl fmt::Display for OperationCatalogueLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroActiveLimit => formatter.write_str("operation active limit must be nonzero"),
            Self::TooManyOperations => write!(
                formatter,
                "operation active plus terminal limit exceeds {MAXIMUM_RETAINED_OPERATIONS}"
            ),
            Self::EncodedWeightTooLarge { maximum, actual } => write!(
                formatter,
                "operation terminal metadata limit is {actual}; maximum is {maximum}"
            ),
        }
    }
}

impl Error for OperationCatalogueLimitsError {}
