use std::{error::Error, fmt, time::Duration};

/// Hard ceiling for root enumeration across listing and policy construction.
pub(crate) const HARD_MAX_SCAN_ENTRIES: usize = 100_000;

/// Optional milestone tier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MilestoneRetention {
    pub(crate) interval: Duration,
    pub(crate) buckets: usize,
}

impl MilestoneRetention {
    /// Keeps the newest candidate in each of a bounded number of age buckets.
    pub fn new(interval: Duration, buckets: usize) -> Result<Self, BackupRetentionPolicyError> {
        if interval.is_zero() || buckets == 0 {
            return Err(BackupRetentionPolicyError::Zero);
        }
        Ok(Self { interval, buckets })
    }
}

/// Deterministic count, age, milestone, and scan bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackupRetentionPolicy {
    pub(crate) keep_newest: usize,
    pub(crate) max_age: Option<Duration>,
    pub(crate) milestones: Option<MilestoneRetention>,
    pub(crate) max_scan_entries: usize,
}

impl BackupRetentionPolicy {
    /// Constructs a finite retention policy.
    pub fn new(
        keep_newest: usize,
        max_age: Option<Duration>,
        milestones: Option<MilestoneRetention>,
        max_scan_entries: usize,
    ) -> Result<Self, BackupRetentionPolicyError> {
        if max_scan_entries == 0 || max_age.is_some_and(|age| age.is_zero()) {
            return Err(BackupRetentionPolicyError::Zero);
        }
        if max_scan_entries > HARD_MAX_SCAN_ENTRIES {
            return Err(BackupRetentionPolicyError::ScanHardCeiling);
        }
        Ok(Self {
            keep_newest,
            max_age,
            milestones,
            max_scan_entries,
        })
    }

    /// Returns the root enumeration bound.
    #[must_use]
    pub const fn max_scan_entries(self) -> usize {
        self.max_scan_entries
    }
}

/// Invalid retention policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupRetentionPolicyError {
    /// A configured interval, bucket count, or scan bound is zero.
    Zero,
    /// Root enumeration bound exceeds the library ceiling.
    ScanHardCeiling,
}

impl fmt::Display for BackupRetentionPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("retention bounds must be non-zero"),
            Self::ScanHardCeiling => formatter.write_str("retention scan exceeds hard ceiling"),
        }
    }
}

impl Error for BackupRetentionPolicyError {}
