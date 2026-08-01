use std::{error::Error, fmt};

/// Defensive ceiling for one notification title.
pub const MAXIMUM_NOTIFICATION_TITLE_BYTES: usize = 4_096;
/// Defensive ceiling for one notification summary.
pub const MAXIMUM_NOTIFICATION_SUMMARY_BYTES: usize = 16_384;
/// Defensive ceiling for one semantic action label.
pub const MAXIMUM_NOTIFICATION_ACTION_LABEL_BYTES: usize = 4_096;
/// Defensive ceiling for actions attached to one notification.
pub const MAXIMUM_NOTIFICATION_ACTIONS: usize = 32;
/// Defensive ceiling for retained records in one process-local ledger.
pub const MAXIMUM_RETAINED_NOTIFICATIONS: usize = 65_536;
/// Defensive ceiling for encoded metadata retained by one ledger.
pub const MAXIMUM_NOTIFICATION_ENCODED_WEIGHT: u64 = 1 << 40;
/// Defensive ceiling for one notification page.
pub const MAXIMUM_NOTIFICATION_PAGE_SIZE: usize = 1_024;

macro_rules! bounded_text {
    ($name:ident, $error:ident, $maximum:ident, $noun:literal) => {
        #[doc = concat!("Nonempty bounded ", $noun, ".")]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Validates and constructs a ", $noun, ".")]
            pub fn new(value: impl Into<String>) -> Result<Self, $error> {
                let value = value.into();
                if value.is_empty() {
                    return Err($error::Empty);
                }
                if value.len() > $maximum {
                    return Err($error::TooLong {
                        maximum: $maximum,
                        actual: value.len(),
                    });
                }
                Ok(Self(value))
            }

            #[doc = concat!("Returns the ", $noun, ".")]
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        #[doc = concat!("Invalid ", $noun, ".")]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $error {
            /// The text was empty.
            Empty,
            /// The text exceeded its hard byte ceiling.
            TooLong {
                /// Hard byte ceiling.
                maximum: usize,
                /// Supplied byte count.
                actual: usize,
            },
        }

        impl fmt::Display for $error {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::Empty => write!(formatter, concat!($noun, " cannot be empty")),
                    Self::TooLong { maximum, actual } => write!(
                        formatter,
                        concat!($noun, " is {} bytes; maximum is {}"),
                        actual, maximum
                    ),
                }
            }
        }

        impl Error for $error {}
    };
}

bounded_text!(
    NotificationTitle,
    NotificationTitleError,
    MAXIMUM_NOTIFICATION_TITLE_BYTES,
    "notification title"
);
bounded_text!(
    NotificationSummary,
    NotificationSummaryError,
    MAXIMUM_NOTIFICATION_SUMMARY_BYTES,
    "notification summary"
);
bounded_text!(
    NotificationActionLabel,
    NotificationActionLabelError,
    MAXIMUM_NOTIFICATION_ACTION_LABEL_BYTES,
    "notification action label"
);

/// Explicit finite bounds for one notification ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotificationLedgerLimits {
    maximum_notifications: usize,
    maximum_encoded_weight: u64,
}

impl NotificationLedgerLimits {
    /// Default process-local ledger bounds.
    pub const DEFAULT: Self = Self {
        maximum_notifications: 500,
        maximum_encoded_weight: 32 * 1_024 * 1_024,
    };

    /// Validates and constructs ledger limits. Zero is a valid closed capacity.
    pub const fn new(
        maximum_notifications: usize,
        maximum_encoded_weight: u64,
    ) -> Result<Self, NotificationLedgerLimitsError> {
        if maximum_notifications > MAXIMUM_RETAINED_NOTIFICATIONS {
            return Err(NotificationLedgerLimitsError::TooManyNotifications {
                maximum: MAXIMUM_RETAINED_NOTIFICATIONS,
                actual: maximum_notifications,
            });
        }
        if maximum_encoded_weight > MAXIMUM_NOTIFICATION_ENCODED_WEIGHT {
            return Err(NotificationLedgerLimitsError::EncodedWeightTooLarge {
                maximum: MAXIMUM_NOTIFICATION_ENCODED_WEIGHT,
                actual: maximum_encoded_weight,
            });
        }
        Ok(Self {
            maximum_notifications,
            maximum_encoded_weight,
        })
    }

    /// Returns the maximum retained record count.
    #[must_use]
    pub const fn maximum_notifications(self) -> usize {
        self.maximum_notifications
    }

    /// Returns the maximum retained encoded metadata weight.
    #[must_use]
    pub const fn maximum_encoded_weight(self) -> u64 {
        self.maximum_encoded_weight
    }
}

/// Invalid notification ledger limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationLedgerLimitsError {
    /// The record count exceeded the defensive ceiling.
    TooManyNotifications {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied count.
        actual: usize,
    },
    /// The encoded weight exceeded the defensive ceiling.
    EncodedWeightTooLarge {
        /// Defensive ceiling.
        maximum: u64,
        /// Supplied weight.
        actual: u64,
    },
}

impl fmt::Display for NotificationLedgerLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyNotifications { maximum, actual } => write!(
                formatter,
                "notification count {actual} exceeds maximum {maximum}"
            ),
            Self::EncodedWeightTooLarge { maximum, actual } => write!(
                formatter,
                "notification encoded weight {actual} exceeds maximum {maximum}"
            ),
        }
    }
}

impl Error for NotificationLedgerLimitsError {}

/// Invalid requested notification page size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationPageSizeError {
    /// Empty pages are not valid queries.
    Zero,
    /// The requested page exceeded its defensive ceiling.
    TooLarge {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied page size.
        actual: usize,
    },
}

impl fmt::Display for NotificationPageSizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("notification page size must be nonzero"),
            Self::TooLarge { maximum, actual } => write!(
                formatter,
                "notification page size {actual} exceeds maximum {maximum}"
            ),
        }
    }
}

impl Error for NotificationPageSizeError {}

pub(crate) const fn validate_page_size(size: usize) -> Result<(), NotificationPageSizeError> {
    if size == 0 {
        Err(NotificationPageSizeError::Zero)
    } else if size > MAXIMUM_NOTIFICATION_PAGE_SIZE {
        Err(NotificationPageSizeError::TooLarge {
            maximum: MAXIMUM_NOTIFICATION_PAGE_SIZE,
            actual: size,
        })
    } else {
        Ok(())
    }
}
