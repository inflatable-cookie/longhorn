use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{BackupArchiveLimits, BackupKind, Sha256Digest, backup::types::UtcTimestamp};

use super::publication::read_bounded_archive;

mod listing;

pub use listing::list_operational_backups;

const HARD_MAX_SCAN_ENTRIES: usize = 100_000;

/// One successfully inspected same-app archive eligible for retention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupOperationalCandidate {
    path: PathBuf,
    archive_id: String,
    created_at: String,
    created_timestamp: UtcTimestamp,
    kind: BackupKind,
    archive_sha256: Sha256Digest,
}

impl BackupOperationalCandidate {
    /// Returns the root-level archive path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the manifest archive id.
    #[must_use]
    pub fn archive_id(&self) -> &str {
        &self.archive_id
    }

    /// Returns the strict manifest UTC creation time.
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Returns the manifest backup kind.
    #[must_use]
    pub const fn kind(&self) -> BackupKind {
        self.kind
    }

    /// Returns SHA-256 over the complete archive.
    #[must_use]
    pub fn archive_sha256(&self) -> &Sha256Digest {
        &self.archive_sha256
    }
}

/// Why one root entry was preserved outside automatic retention.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BackupRetentionDiagnosticKind {
    /// Entry is not a plaintext Longhorn archive.
    Unmanaged,
    /// Encrypted archive cannot be inspected by the plaintext layer.
    Locked,
    /// Candidate could not be read.
    Unreadable,
    /// Candidate is not a regular file.
    NonRegular,
    /// Plaintext archive is malformed or damaged.
    Corrupt,
    /// Archive uses a future or otherwise unsupported format.
    UnknownFormat,
    /// Archive belongs to another application.
    ForeignApplication,
    /// User-export archive was placed in the operational root.
    UserExport,
    /// More than one valid candidate claims the same archive id.
    DuplicateArchiveId,
    /// Root enumeration exceeded its explicit bound.
    ScanLimit,
    /// Reading the root itself failed.
    RootRead,
    /// The just-published archive predates another valid manifest.
    ClockRegression,
    /// A requested pin was not present in the complete listing.
    MissingPin,
    /// The just-published archive was not present in the complete listing.
    MissingNewArchive,
}

/// Non-fatal listing or retention evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionDiagnostic {
    /// Diagnostic class.
    pub kind: BackupRetentionDiagnosticKind,
    /// Affected root entry when one exists.
    pub path: Option<PathBuf>,
    /// Stable human-readable detail.
    pub detail: String,
}

/// Bounded operational-root inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupOperationalListing {
    root: PathBuf,
    candidates: Vec<BackupOperationalCandidate>,
    diagnostics: Vec<BackupRetentionDiagnostic>,
    complete: bool,
}

impl BackupOperationalListing {
    /// Returns the exact operational root that was inspected.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns proven same-app retention candidates in newest-first order.
    #[must_use]
    pub fn candidates(&self) -> &[BackupOperationalCandidate] {
        &self.candidates
    }

    /// Returns preserved-entry and scan diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[BackupRetentionDiagnostic] {
        &self.diagnostics
    }

    /// Reports whether the root was enumerated completely.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }
}

/// Optional milestone tier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MilestoneRetention {
    interval: Duration,
    buckets: usize,
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
    keep_newest: usize,
    max_age: Option<Duration>,
    milestones: Option<MilestoneRetention>,
    max_scan_entries: usize,
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
    root: PathBuf,
    deletions: Vec<BackupRetentionDeletion>,
    retained: BTreeMap<Sha256Digest, BTreeSet<BackupRetentionReason>>,
    diagnostics: Vec<BackupRetentionDiagnostic>,
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

/// Applied deletion receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionApplyReceipt {
    /// Exact paths removed.
    pub deleted: Vec<PathBuf>,
}

/// Failure while rechecking or applying one exact deletion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionApplyError {
    /// Candidate path.
    pub path: PathBuf,
    /// Whether this candidate was already removed.
    pub deleted: bool,
    /// Failure detail.
    pub detail: String,
}

impl fmt::Display for BackupRetentionApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot apply backup retention to {}: {}",
            self.path.display(),
            self.detail
        )
    }
}

impl Error for BackupRetentionApplyError {}

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

/// Rechecks exact bytes and removes only unchanged planned candidates.
pub fn apply_backup_retention(
    plan: &BackupRetentionPlan,
    archive_limits: BackupArchiveLimits,
) -> Result<BackupRetentionApplyReceipt, BackupRetentionApplyError> {
    let mut deleted = Vec::new();
    for deletion in &plan.deletions {
        if deletion.path.parent() != Some(plan.root.as_path()) {
            return Err(apply_error(
                deletion.path.clone(),
                false,
                "planned path escaped the operational root",
            ));
        }
        let bytes = read_bounded_archive(&deletion.path, archive_limits)
            .map_err(|error| apply_error(deletion.path.clone(), false, error.to_string()))?;
        if Sha256Digest::from_bytes(&bytes) != deletion.archive_sha256 {
            return Err(apply_error(
                deletion.path.clone(),
                false,
                "archive changed after retention planning",
            ));
        }
        fs::remove_file(&deletion.path)
            .map_err(|error| apply_error(deletion.path.clone(), false, error.to_string()))?;
        deleted.push(deletion.path.clone());
    }
    Ok(BackupRetentionApplyReceipt { deleted })
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

fn diagnostic(
    kind: BackupRetentionDiagnosticKind,
    path: Option<PathBuf>,
    detail: impl Into<String>,
) -> BackupRetentionDiagnostic {
    BackupRetentionDiagnostic {
        kind,
        path,
        detail: detail.into(),
    }
}

fn apply_error(
    path: PathBuf,
    deleted: bool,
    detail: impl Into<String>,
) -> BackupRetentionApplyError {
    BackupRetentionApplyError {
        path,
        deleted,
        detail: detail.into(),
    }
}
