use std::{error::Error, fmt};

use crate::MAXIMUM_HISTORY_LABEL_BYTES;

/// Defensive hard ceiling for retained entries before retention policy exists.
pub const MAXIMUM_HISTORY_ENTRIES: usize = 65_536;
/// Defensive hard ceiling for retained encoded payload weight.
pub const MAXIMUM_HISTORY_ENCODED_WEIGHT: u64 = 1 << 40;
/// Defensive hard ceiling for one navigation batch.
pub const MAXIMUM_HISTORY_NAVIGATION_STEPS: usize = 65_536;
/// Defensive hard ceiling for recently committed plan identities.
pub const MAXIMUM_RECENT_HISTORY_PLANS: usize = 65_536;
/// Defensive hard ceiling for one metadata projection page.
pub const MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE: usize = 4_096;

/// Explicit count and metadata limits for one linear history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryLimits {
    maximum_entries: usize,
    maximum_encoded_weight: u64,
    maximum_label_bytes: usize,
}

impl HistoryLimits {
    /// Validates and constructs limits.
    pub const fn new(
        maximum_entries: usize,
        maximum_encoded_weight: u64,
        maximum_label_bytes: usize,
    ) -> Result<Self, HistoryLimitsError> {
        if maximum_entries == 0 || maximum_encoded_weight == 0 || maximum_label_bytes == 0 {
            return Err(HistoryLimitsError::Zero);
        }
        if maximum_entries > MAXIMUM_HISTORY_ENTRIES {
            return Err(HistoryLimitsError::TooManyEntries {
                maximum: MAXIMUM_HISTORY_ENTRIES,
                actual: maximum_entries,
            });
        }
        if maximum_encoded_weight > MAXIMUM_HISTORY_ENCODED_WEIGHT {
            return Err(HistoryLimitsError::EncodedWeightTooLarge {
                maximum: MAXIMUM_HISTORY_ENCODED_WEIGHT,
                actual: maximum_encoded_weight,
            });
        }
        if maximum_label_bytes > MAXIMUM_HISTORY_LABEL_BYTES {
            return Err(HistoryLimitsError::LabelBytesTooLarge {
                maximum: MAXIMUM_HISTORY_LABEL_BYTES,
                actual: maximum_label_bytes,
            });
        }
        Ok(Self {
            maximum_entries,
            maximum_encoded_weight,
            maximum_label_bytes,
        })
    }

    /// Returns the maximum entries across applied and future state.
    #[must_use]
    pub const fn maximum_entries(self) -> usize {
        self.maximum_entries
    }

    /// Returns the maximum total consumer-measured encoded payload weight.
    #[must_use]
    pub const fn maximum_encoded_weight(self) -> u64 {
        self.maximum_encoded_weight
    }

    /// Returns the configured maximum label byte length.
    #[must_use]
    pub const fn maximum_label_bytes(self) -> usize {
        self.maximum_label_bytes
    }
}

impl Default for HistoryLimits {
    fn default() -> Self {
        Self {
            maximum_entries: 100,
            maximum_encoded_weight: MAXIMUM_HISTORY_ENCODED_WEIGHT,
            maximum_label_bytes: 1_024,
        }
    }
}

/// Invalid history limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryLimitsError {
    /// A configured limit was zero.
    Zero,
    /// The entry limit exceeded the defensive ceiling.
    TooManyEntries {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
    /// The encoded-weight limit exceeded the defensive ceiling.
    EncodedWeightTooLarge {
        /// Defensive ceiling.
        maximum: u64,
        /// Supplied limit.
        actual: u64,
    },
    /// The label-byte limit exceeded the defensive ceiling.
    LabelBytesTooLarge {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
}

impl fmt::Display for HistoryLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("history limits must be nonzero"),
            Self::TooManyEntries { maximum, actual } => write!(
                formatter,
                "history entry limit is {actual}; hard maximum is {maximum}"
            ),
            Self::EncodedWeightTooLarge { maximum, actual } => write!(
                formatter,
                "history encoded-weight limit is {actual}; hard maximum is {maximum}"
            ),
            Self::LabelBytesTooLarge { maximum, actual } => write!(
                formatter,
                "history label limit is {actual} bytes; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryLimitsError {}

/// Explicit metadata-page size limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryProjectionLimits {
    maximum_page_size: usize,
}

impl HistoryProjectionLimits {
    /// Default projection limits.
    pub const DEFAULT: Self = Self {
        maximum_page_size: 100,
    };

    /// Validates and constructs projection limits.
    pub const fn new(maximum_page_size: usize) -> Result<Self, HistoryProjectionLimitsError> {
        if maximum_page_size == 0 {
            return Err(HistoryProjectionLimitsError::Zero);
        }
        if maximum_page_size > MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE {
            return Err(HistoryProjectionLimitsError::TooLarge {
                maximum: MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
                actual: maximum_page_size,
            });
        }
        Ok(Self { maximum_page_size })
    }

    /// Returns the maximum entries in one metadata page.
    #[must_use]
    pub const fn maximum_page_size(self) -> usize {
        self.maximum_page_size
    }
}

impl Default for HistoryProjectionLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Invalid history projection limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryProjectionLimitsError {
    /// The configured page size was zero.
    Zero,
    /// The page size exceeded the defensive ceiling.
    TooLarge {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
}

impl fmt::Display for HistoryProjectionLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("history projection page size must be nonzero"),
            Self::TooLarge { maximum, actual } => write!(
                formatter,
                "history projection page size is {actual}; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryProjectionLimitsError {}

/// Bounded navigation-batch and duplicate-plan tracking limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryNavigationLimits {
    maximum_steps: usize,
    maximum_recent_plans: usize,
}

impl HistoryNavigationLimits {
    /// Default navigation limits.
    pub const DEFAULT: Self = Self {
        maximum_steps: 512,
        maximum_recent_plans: 1_024,
    };

    /// Validates and constructs navigation limits.
    pub const fn new(
        maximum_steps: usize,
        maximum_recent_plans: usize,
    ) -> Result<Self, HistoryNavigationLimitsError> {
        if maximum_steps == 0 || maximum_recent_plans == 0 {
            return Err(HistoryNavigationLimitsError::Zero);
        }
        if maximum_steps > MAXIMUM_HISTORY_NAVIGATION_STEPS {
            return Err(HistoryNavigationLimitsError::TooManySteps {
                maximum: MAXIMUM_HISTORY_NAVIGATION_STEPS,
                actual: maximum_steps,
            });
        }
        if maximum_recent_plans > MAXIMUM_RECENT_HISTORY_PLANS {
            return Err(HistoryNavigationLimitsError::TooManyRecentPlans {
                maximum: MAXIMUM_RECENT_HISTORY_PLANS,
                actual: maximum_recent_plans,
            });
        }
        Ok(Self {
            maximum_steps,
            maximum_recent_plans,
        })
    }

    /// Returns the maximum payload steps in one navigation.
    #[must_use]
    pub const fn maximum_steps(self) -> usize {
        self.maximum_steps
    }

    /// Returns the maximum recent committed plan ids retained for deduplication.
    #[must_use]
    pub const fn maximum_recent_plans(self) -> usize {
        self.maximum_recent_plans
    }
}

impl Default for HistoryNavigationLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Invalid history navigation limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryNavigationLimitsError {
    /// A configured limit was zero.
    Zero,
    /// The step limit exceeded the defensive ceiling.
    TooManySteps {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
    /// The recent-plan limit exceeded the defensive ceiling.
    TooManyRecentPlans {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied limit.
        actual: usize,
    },
}

impl fmt::Display for HistoryNavigationLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("history navigation limits must be nonzero"),
            Self::TooManySteps { maximum, actual } => write!(
                formatter,
                "history navigation step limit is {actual}; hard maximum is {maximum}"
            ),
            Self::TooManyRecentPlans { maximum, actual } => write!(
                formatter,
                "history recent-plan limit is {actual}; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryNavigationLimitsError {}
