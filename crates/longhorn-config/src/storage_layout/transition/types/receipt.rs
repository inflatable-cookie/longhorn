use std::path::PathBuf;

use longhorn_core::DomainId;

use crate::{Sha256Digest, StorageFileEvidence};

/// Verified authority after transition or recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageTransitionOutcome {
    /// Fixed locator selects the verified target.
    TargetCommitted,
    /// Fixed locator still selects the verified source.
    SourceRetained,
}

/// Successful locator-last transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionReceipt {
    pub(crate) transition_id: String,
    pub(crate) outcome: StorageTransitionOutcome,
    pub(crate) target_layout_digest: Sha256Digest,
    pub(crate) copied_domains: Vec<DomainId>,
    pub(crate) custom_domains: Vec<DomainId>,
    pub(crate) retained_source_paths: Vec<PathBuf>,
    pub(crate) retained_source_evidence: Vec<StorageFileEvidence>,
    pub(crate) receipt_digest: Sha256Digest,
}

impl StorageTransitionReceipt {
    /// Returns transition id.
    #[must_use]
    pub fn transition_id(&self) -> &str {
        &self.transition_id
    }
    /// Returns terminal authority.
    #[must_use]
    pub const fn outcome(&self) -> StorageTransitionOutcome {
        self.outcome
    }
    /// Returns target layout evidence.
    #[must_use]
    pub fn target_layout_digest(&self) -> &Sha256Digest {
        &self.target_layout_digest
    }
    /// Returns copied ordinary domains.
    #[must_use]
    pub fn copied_domains(&self) -> &[DomainId] {
        &self.copied_domains
    }
    /// Returns adapter-restored domains.
    #[must_use]
    pub fn custom_domains(&self) -> &[DomainId] {
        &self.custom_domains
    }
    /// Returns registered source files retained after commit.
    #[must_use]
    pub fn retained_source_paths(&self) -> &[PathBuf] {
        &self.retained_source_paths
    }
    /// Returns receipt evidence used to bind cleanup.
    #[must_use]
    pub fn receipt_digest(&self) -> &Sha256Digest {
        &self.receipt_digest
    }
}

/// Recovery of one interrupted transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionRecoveryReceipt {
    pub(crate) transition_id: String,
    pub(crate) outcome: StorageTransitionOutcome,
}

impl StorageTransitionRecoveryReceipt {
    /// Returns recovered transition id.
    #[must_use]
    pub fn transition_id(&self) -> &str {
        &self.transition_id
    }
    /// Returns recovered authority.
    #[must_use]
    pub const fn outcome(&self) -> StorageTransitionOutcome {
        self.outcome
    }
}

/// Receipt-bound source cleanup plan. It never includes unknown files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionCleanupPlan {
    pub(crate) transition_id: String,
    pub(crate) receipt_digest: Sha256Digest,
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) evidence: Vec<StorageFileEvidence>,
}

impl StorageTransitionCleanupPlan {
    /// Builds cleanup only from a committed receipt.
    #[must_use]
    pub fn from_receipt(receipt: &StorageTransitionReceipt) -> Option<Self> {
        (receipt.outcome == StorageTransitionOutcome::TargetCommitted).then(|| Self {
            transition_id: receipt.transition_id.clone(),
            receipt_digest: receipt.receipt_digest.clone(),
            paths: receipt.retained_source_paths.clone(),
            evidence: receipt.retained_source_evidence.clone(),
        })
    }

    /// Returns bound transition id.
    #[must_use]
    pub fn transition_id(&self) -> &str {
        &self.transition_id
    }

    /// Returns bound receipt evidence.
    #[must_use]
    pub fn receipt_digest(&self) -> &Sha256Digest {
        &self.receipt_digest
    }

    /// Returns exact registered source paths eligible for deletion.
    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

/// Idempotent result of applying a receipt-bound source cleanup plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionCleanupReceipt {
    pub(crate) transition_id: String,
    pub(crate) deleted_paths: Vec<PathBuf>,
    pub(crate) already_absent_paths: Vec<PathBuf>,
}

impl StorageTransitionCleanupReceipt {
    /// Returns the transition whose retained source was cleaned.
    #[must_use]
    pub fn transition_id(&self) -> &str {
        &self.transition_id
    }

    /// Returns exact registered paths deleted by this call.
    #[must_use]
    pub fn deleted_paths(&self) -> &[PathBuf] {
        &self.deleted_paths
    }

    /// Returns exact registered paths already absent when this call ran.
    #[must_use]
    pub fn already_absent_paths(&self) -> &[PathBuf] {
        &self.already_absent_paths
    }
}
