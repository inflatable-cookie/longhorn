use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    path::PathBuf,
    time::Duration,
};

use crate::{Sha256Digest, backup::types::UtcTimestamp};

use super::{
    BackupOperationalCandidate, BackupOperationalListing, BackupRetentionDiagnostic,
    BackupRetentionDiagnosticKind, BackupRetentionPolicy, diagnostic,
};

/// Why a proven archive is retained.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BackupRetentionReason {
    /// Archive was just published.
    NewArchive,
    /// Owning operation pinned the archive.
    Pinned,
    /// Archive is inside the newest-count tier.
    NewestCount,
    /// Archive is inside the age tier.
    Age,
    /// Archive represents a milestone bucket.
    Milestone,
}

/// One exact proven deletion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionDeletion {
    /// Root-level candidate path.
    pub path: PathBuf,
    /// Hash that must still match before deletion.
    pub archive_sha256: Sha256Digest,
}

/// Side-effect-free retention decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionPlan {
    pub(crate) root: PathBuf,
    pub(crate) deletions: Vec<BackupRetentionDeletion>,
    pub(crate) retained: BTreeMap<Sha256Digest, BTreeSet<BackupRetentionReason>>,
    pub(crate) diagnostics: Vec<BackupRetentionDiagnostic>,
}

impl BackupRetentionPlan {
    /// Returns exact candidates selected for deletion.
    #[must_use]
    pub fn deletions(&self) -> &[BackupRetentionDeletion] {
        &self.deletions
    }

    /// Returns keep reasons keyed by complete archive hash.
    #[must_use]
    pub fn retained(&self) -> &BTreeMap<Sha256Digest, BTreeSet<BackupRetentionReason>> {
        &self.retained
    }

    /// Returns retention-specific diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[BackupRetentionDiagnostic] {
        &self.diagnostics
    }
}

/// Retention planning refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupRetentionPlanError {
    /// Incomplete enumeration makes pruning unsafe.
    IncompleteListing,
}

impl fmt::Display for BackupRetentionPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cannot prune from an incomplete backup listing")
    }
}

impl Error for BackupRetentionPlanError {}

/// Selects exact deletions from a complete listing.
pub fn plan_backup_retention(
    listing: &BackupOperationalListing,
    policy: BackupRetentionPolicy,
    pins: &BTreeSet<Sha256Digest>,
    new_archive: &Sha256Digest,
) -> Result<BackupRetentionPlan, BackupRetentionPlanError> {
    if !listing.complete {
        return Err(BackupRetentionPlanError::IncompleteListing);
    }
    let mut retained = BTreeMap::<Sha256Digest, BTreeSet<BackupRetentionReason>>::new();
    let mut diagnostics = Vec::new();
    for candidate in listing.candidates.iter().take(policy.keep_newest) {
        keep(&mut retained, candidate, BackupRetentionReason::NewestCount);
    }

    let newest_time = listing
        .candidates
        .first()
        .map(|candidate| candidate.created_timestamp);
    if let (Some(max_age), Some(newest_time)) = (policy.max_age, newest_time) {
        for candidate in &listing.candidates {
            if elapsed(newest_time, candidate.created_timestamp) <= max_age {
                keep(&mut retained, candidate, BackupRetentionReason::Age);
            }
        }
    }
    if let (Some(milestones), Some(newest_time)) = (policy.milestones, newest_time) {
        let interval_nanos = duration_nanos(milestones.interval);
        let mut used = BTreeSet::new();
        for candidate in &listing.candidates {
            let bucket =
                timestamp_distance_nanos(newest_time, candidate.created_timestamp) / interval_nanos;
            if bucket < milestones.buckets as u128 && used.insert(bucket) {
                keep(&mut retained, candidate, BackupRetentionReason::Milestone);
            }
        }
    }

    let new_candidate = listing
        .candidates
        .iter()
        .find(|candidate| candidate.archive_sha256 == *new_archive);
    if let Some(candidate) = new_candidate {
        keep(&mut retained, candidate, BackupRetentionReason::NewArchive);
        if listing.candidates.iter().any(|other| {
            other.archive_sha256 != *new_archive
                && other.created_timestamp > candidate.created_timestamp
        }) {
            diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::ClockRegression,
                Some(candidate.path.clone()),
                "new archive creation time predates an existing valid archive",
            ));
        }
    } else {
        diagnostics.push(diagnostic(
            BackupRetentionDiagnosticKind::MissingNewArchive,
            None,
            format!("new archive {} is absent", new_archive.as_str()),
        ));
    }
    for pin in pins {
        if let Some(candidate) = listing
            .candidates
            .iter()
            .find(|candidate| candidate.archive_sha256 == *pin)
        {
            keep(&mut retained, candidate, BackupRetentionReason::Pinned);
        } else {
            diagnostics.push(diagnostic(
                BackupRetentionDiagnosticKind::MissingPin,
                None,
                format!("pinned archive {} is absent", pin.as_str()),
            ));
        }
    }

    let deletions = listing
        .candidates
        .iter()
        .filter(|candidate| !retained.contains_key(&candidate.archive_sha256))
        .map(|candidate| BackupRetentionDeletion {
            path: candidate.path.clone(),
            archive_sha256: candidate.archive_sha256.clone(),
        })
        .collect();
    Ok(BackupRetentionPlan {
        root: listing.root.clone(),
        deletions,
        retained,
        diagnostics,
    })
}

fn keep(
    retained: &mut BTreeMap<Sha256Digest, BTreeSet<BackupRetentionReason>>,
    candidate: &BackupOperationalCandidate,
    reason: BackupRetentionReason,
) {
    retained
        .entry(candidate.archive_sha256.clone())
        .or_default()
        .insert(reason);
}

fn elapsed(newest: UtcTimestamp, candidate: UtcTimestamp) -> Duration {
    let nanos = timestamp_distance_nanos(newest, candidate);
    Duration::new(
        u64::try_from(nanos / 1_000_000_000).unwrap_or(u64::MAX),
        (nanos % 1_000_000_000) as u32,
    )
}

fn timestamp_distance_nanos(newest: UtcTimestamp, candidate: UtcTimestamp) -> u128 {
    let seconds = newest.seconds.saturating_sub(candidate.seconds) as u128;
    let newest_nanos = seconds
        .saturating_mul(1_000_000_000)
        .saturating_add(u128::from(newest.nanoseconds));
    newest_nanos.saturating_sub(u128::from(candidate.nanoseconds))
}

fn duration_nanos(duration: Duration) -> u128 {
    u128::from(duration.as_secs())
        .saturating_mul(1_000_000_000)
        .saturating_add(u128::from(duration.subsec_nanos()))
}
