use std::{error::Error, fmt};

use longhorn_core::DomainId;

use crate::{CoordinationFailure, DomainLocation, RecoveryKind, backup::BackupAdapterError};

/// Failure to produce one complete bounded coordinated snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupCaptureError {
    /// A catalogue entry does not belong to this store.
    CatalogDomainNotRegistered {
        /// Unregistered domain.
        domain: DomainId,
    },
    /// A selected scope domain is not registered.
    ScopeDomainNotRegistered {
        /// Unregistered selected domain.
        domain: DomainId,
    },
    /// A selected domain has no explicit or safe default policy.
    MissingPolicy {
        /// Domain requiring policy.
        domain: DomainId,
    },
    /// A registered descriptor changed after policy declaration.
    DescriptorChanged {
        /// Changed domain.
        domain: DomainId,
    },
    /// Adapter execution failed through its declared authority.
    AdapterFailed {
        /// Delegated domain.
        domain: DomainId,
        /// Stable adapter id.
        adapter: String,
        /// Stable adapter failure.
        error: BackupAdapterError,
    },
    /// Guarded capture cannot run an independent external transaction.
    ExternalAdapterRequiresUnlockedCapture {
        /// Delegated domain.
        domain: DomainId,
        /// Stable adapter id.
        adapter: String,
    },
    /// Adapter returned malformed, duplicate, or unbounded payload evidence.
    InvalidAdapterCapture {
        /// Delegated domain.
        domain: DomainId,
        /// Stable adapter id.
        adapter: String,
        /// Validation detail.
        detail: String,
    },
    /// Two adapters declared different authority for one group id.
    ConsistencyGroupConflict {
        /// Conflicting group id.
        group: String,
    },
    /// Included domain has no ordinary file authority.
    Unavailable {
        /// Unavailable domain.
        domain: DomainId,
        /// Required location.
        location: DomainLocation,
    },
    /// Store coordination failed.
    Coordination(CoordinationFailure),
    /// Required source could not be read.
    Unreadable {
        /// Unreadable domain.
        domain: DomainId,
        /// I/O detail.
        detail: String,
    },
    /// One source exceeded its configured bound.
    DomainTooLarge {
        /// Oversized domain.
        domain: DomainId,
        /// Configured byte limit.
        limit: usize,
        /// Observed bytes.
        observed: usize,
    },
    /// Complete retained bytes exceeded their configured bound.
    TotalTooLarge {
        /// Domain that crossed the bound.
        domain: DomainId,
        /// Configured aggregate byte limit.
        limit: usize,
        /// Observed total bytes.
        observed: usize,
    },
    /// Checked aggregate byte arithmetic overflowed.
    TotalSizeOverflow {
        /// Domain being added.
        domain: DomainId,
    },
    /// A typed load returned a recovery state without preservable source.
    UnclassifiedRecovery {
        /// Affected domain.
        domain: DomainId,
        /// Recovery category.
        kind: RecoveryKind,
        /// Recovery detail.
        detail: String,
    },
}

impl fmt::Display for BackupCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CatalogDomainNotRegistered { domain } => {
                write!(
                    formatter,
                    "backup catalogue domain {domain} is not registered"
                )
            }
            Self::ScopeDomainNotRegistered { domain } => {
                write!(formatter, "backup scope domain {domain} is not registered")
            }
            Self::MissingPolicy { domain } => {
                write!(formatter, "backup domain {domain} has no policy")
            }
            Self::DescriptorChanged { domain } => {
                write!(
                    formatter,
                    "backup domain {domain} changed after policy declaration"
                )
            }
            Self::AdapterFailed {
                domain,
                adapter,
                error,
            } => {
                write!(
                    formatter,
                    "backup adapter {adapter} failed for domain {domain}: {error}"
                )
            }
            Self::ExternalAdapterRequiresUnlockedCapture { domain, adapter } => write!(
                formatter,
                "external backup adapter {adapter} for domain {domain} cannot run under the Longhorn coordinator"
            ),
            Self::InvalidAdapterCapture {
                domain,
                adapter,
                detail,
            } => write!(
                formatter,
                "backup adapter {adapter} returned invalid capture for {domain}: {detail}"
            ),
            Self::ConsistencyGroupConflict { group } => {
                write!(
                    formatter,
                    "backup consistency group {group} has conflicting authority"
                )
            }
            Self::Unavailable { domain, location } => {
                write!(
                    formatter,
                    "backup domain {domain} is unavailable at {location:?}"
                )
            }
            Self::Coordination(error) => error.fmt(formatter),
            Self::Unreadable { domain, detail } => {
                write!(formatter, "backup domain {domain} is unreadable: {detail}")
            }
            Self::DomainTooLarge {
                domain,
                limit,
                observed,
            } => write!(
                formatter,
                "backup domain {domain} has {observed} bytes; limit is {limit}"
            ),
            Self::TotalTooLarge {
                domain,
                limit,
                observed,
            } => write!(
                formatter,
                "backup total reached {observed} bytes while adding {domain}; limit is {limit}"
            ),
            Self::TotalSizeOverflow { domain } => {
                write!(
                    formatter,
                    "backup byte total overflowed while adding {domain}"
                )
            }
            Self::UnclassifiedRecovery {
                domain,
                kind,
                detail,
            } => write!(
                formatter,
                "backup domain {domain} has unclassified {kind:?} recovery: {detail}"
            ),
        }
    }
}

impl Error for BackupCaptureError {}
