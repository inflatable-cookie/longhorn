mod transition;

use longhorn_storage_layout::StorageLayoutDigest;
pub use transition::{
    LegacyStorageCandidate, LegacyStorageDiscovery, StorageFileEvidence, StorageTransitionAction,
    StorageTransitionAdapter, StorageTransitionCatalog, StorageTransitionCleanupPlan,
    StorageTransitionCleanupReceipt, StorageTransitionConflict, StorageTransitionConflictKind,
    StorageTransitionDomain, StorageTransitionError, StorageTransitionExclusion,
    StorageTransitionExecutionOptions, StorageTransitionGuard, StorageTransitionLimits,
    StorageTransitionOutcome, StorageTransitionPlan, StorageTransitionPlanError,
    StorageTransitionPreview, StorageTransitionReceipt, StorageTransitionRecoveryReceipt,
    StorageTransitionRequest, StorageTransitionUnknownFile, apply_storage_transition_cleanup,
    discover_legacy_storage, execute_storage_transition, inspect_storage_transition,
    plan_storage_transition, recover_storage_transition,
};

/// Converts resolver layout evidence into the config store's digest type.
///
/// Both types carry the same validated lowercase hexadecimal SHA-256 string;
/// the layout digest is always produced by this crate's resolver receipt, so
/// the conversion cannot fail.
pub(crate) fn committed_layout_digest(digest: &StorageLayoutDigest) -> crate::Sha256Digest {
    crate::Sha256Digest::new(digest.as_str().to_owned())
        .expect("layout digest is a validated SHA-256 hexadecimal string")
}

/// Converts a committed store digest into the resolver's layout evidence type.
///
/// The store digest was validated when it was parsed or computed, so the
/// conversion cannot fail.
pub(crate) fn layout_evidence(digest: &crate::Sha256Digest) -> StorageLayoutDigest {
    StorageLayoutDigest::new(digest.as_str().to_owned())
        .expect("committed layout digest is a validated SHA-256 hexadecimal string")
}

pub(crate) fn layout_digest_matches(
    store_digest: &crate::Sha256Digest,
    layout_digest: &StorageLayoutDigest,
) -> bool {
    store_digest.as_str() == layout_digest.as_str()
}
