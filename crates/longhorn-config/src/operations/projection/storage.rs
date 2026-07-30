use crate::{
    ConfigOperationProjectionError, PlatformDirectoryFact, RootKind, Sha256Digest, StorageClass,
    StorageFileEvidence, StorageLayoutDiagnostic, StorageLayoutWarning, StorageLeafProvenance,
    StorageProfile, StorageRootProvenance, StorageTransitionAction,
    StorageTransitionCleanupReceipt, StorageTransitionConflictKind, StorageTransitionOutcome,
    StorageTransitionPreview, StorageTransitionReceipt, TargetPlatform,
};

use super::super::{
    StorageCleanupReceiptProjection, StorageLayoutProjection, StorageLeafProvenanceProjection,
    StorageProfileId, StorageRootProjection, StorageTransitionConflictProjection,
    StorageTransitionDomainProjection, StorageTransitionPreviewProjection,
    StorageTransitionReceiptProjection,
};
use super::exact_path;

impl From<StorageProfile> for StorageProfileId {
    fn from(value: StorageProfile) -> Self {
        match value {
            StorageProfile::PlatformNativeV1 => Self::PlatformNativeV1,
            StorageProfile::UnifiedAppRootV1 => Self::UnifiedAppRootV1,
            StorageProfile::PortableV1 => Self::PortableV1,
        }
    }
}

impl TryFrom<&StorageLayoutDiagnostic> for StorageLayoutProjection {
    type Error = ConfigOperationProjectionError;

    fn try_from(value: &StorageLayoutDiagnostic) -> Result<Self, Self::Error> {
        Ok(Self {
            profile: StorageProfile::from_id(value.profile_id())
                .expect("resolved layout contains a built-in profile")
                .into(),
            platform: platform_id(value.platform()).into(),
            canonical_application_id: value.canonical_application_id().into(),
            effective_leaf: value.effective_leaf().into(),
            leaf_provenance: match value.leaf_provenance() {
                StorageLeafProvenance::CanonicalApplicationId => {
                    StorageLeafProvenanceProjection::CanonicalApplicationId
                }
                StorageLeafProvenance::StableStorageName => {
                    StorageLeafProvenanceProjection::StableStorageName
                }
            },
            roots: value
                .roots()
                .iter()
                .map(|root| {
                    Ok(StorageRootProjection {
                        kind: root_kind_id(root.kind()).into(),
                        path: exact_path(root.path())?,
                        provenance: root_provenance_id(root.provenance()),
                    })
                })
                .collect::<Result<_, ConfigOperationProjectionError>>()?,
            warnings: value
                .warnings()
                .iter()
                .map(|warning| warning_id(*warning).into())
                .collect(),
            layout_digest: value.digest().as_str().into(),
        })
    }
}

impl StorageTransitionPreviewProjection {
    /// Projects one exact non-mutating transition inspection.
    pub fn try_from_preview(
        preview: &StorageTransitionPreview,
    ) -> Result<Self, ConfigOperationProjectionError> {
        Ok(Self {
            source_layout_digest: preview.source_layout_digest().as_str().into(),
            target_layout_digest: preview.target_layout_digest().as_str().into(),
            target_profile: preview.target_selection().profile().into(),
            domains: preview
                .domains()
                .iter()
                .map(|domain| {
                    Ok(StorageTransitionDomainProjection {
                        domain_id: domain.domain().as_str().into(),
                        storage_class: storage_class_id(domain.storage_class()).into(),
                        action: transition_action_id(domain.action()),
                        source_path: domain.source_path().map(exact_path).transpose()?,
                        target_path: domain.target_path().map(exact_path).transpose()?,
                        source_sha256: evidence_digest(domain.source_evidence()),
                    })
                })
                .collect::<Result<_, ConfigOperationProjectionError>>()?,
            unknown_source_paths: preview
                .source_unknown()
                .iter()
                .map(|file| exact_path(file.path()))
                .collect::<Result<_, _>>()?,
            conflicts: preview
                .conflicts()
                .iter()
                .map(|conflict| {
                    Ok(StorageTransitionConflictProjection {
                        kind: conflict_kind_id(conflict.kind()).into(),
                        path: conflict.path().map(exact_path).transpose()?,
                        detail: conflict.detail().into(),
                    })
                })
                .collect::<Result<_, ConfigOperationProjectionError>>()?,
            evidence_digest: preview.evidence_digest().as_str().into(),
            confirmation_digest: preview.confirmation_digest().as_str().into(),
        })
    }
}

impl TryFrom<&StorageTransitionReceipt> for StorageTransitionReceiptProjection {
    type Error = ConfigOperationProjectionError;

    fn try_from(value: &StorageTransitionReceipt) -> Result<Self, Self::Error> {
        Ok(Self {
            transition_id: value.transition_id().into(),
            outcome: transition_outcome_id(value.outcome()).into(),
            target_layout_digest: value.target_layout_digest().as_str().into(),
            copied_domain_ids: value
                .copied_domains()
                .iter()
                .map(|domain| domain.as_str().into())
                .collect(),
            custom_domain_ids: value
                .custom_domains()
                .iter()
                .map(|domain| domain.as_str().into())
                .collect(),
            retained_source_paths: value
                .retained_source_paths()
                .iter()
                .map(|path| exact_path(path))
                .collect::<Result<_, _>>()?,
            receipt_digest: value.receipt_digest().as_str().into(),
        })
    }
}

impl StorageCleanupReceiptProjection {
    /// Projects exact idempotent cleanup evidence with its authorizing receipt.
    pub fn try_from_receipt(
        receipt: &StorageTransitionCleanupReceipt,
        transition_receipt_digest: &Sha256Digest,
    ) -> Result<Self, ConfigOperationProjectionError> {
        let removed_paths = receipt
            .deleted_paths()
            .iter()
            .chain(receipt.already_absent_paths())
            .map(|path| exact_path(path))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            transition_id: receipt.transition_id().into(),
            transition_receipt_digest: transition_receipt_digest.as_str().into(),
            removed_paths,
        })
    }
}

const fn platform_id(value: TargetPlatform) -> &'static str {
    match value {
        TargetPlatform::MacOs => "macos",
        TargetPlatform::Windows => "windows",
        TargetPlatform::Linux => "linux",
    }
}

pub(super) const fn root_kind_id(value: RootKind) -> &'static str {
    match value {
        RootKind::Config => "config",
        RootKind::Data => "data",
        RootKind::State => "state",
        RootKind::Cache => "cache",
        RootKind::Runtime => "runtime",
        RootKind::Log => "log",
        RootKind::Backup => "backup",
        RootKind::Policy => "policy",
        RootKind::Workspace => "workspace",
        RootKind::Project => "project",
    }
}

fn root_provenance_id(value: StorageRootProvenance) -> String {
    match value {
        StorageRootProvenance::PlatformProfile(fact) => {
            format!("platform:{}", directory_fact_id(fact))
        }
        StorageRootProvenance::UnifiedProfile(fact) => {
            format!("unified:{}", directory_fact_id(fact))
        }
        StorageRootProvenance::PortableProfile => "portable".into(),
        StorageRootProvenance::DerivedFrom(root) => {
            format!("derived:{}", root_kind_id(root))
        }
        StorageRootProvenance::ExplicitOverride => "override".into(),
    }
}

const fn directory_fact_id(value: PlatformDirectoryFact) -> &'static str {
    match value {
        PlatformDirectoryFact::Config => "config",
        PlatformDirectoryFact::Data => "data",
        PlatformDirectoryFact::State => "state",
        PlatformDirectoryFact::Cache => "cache",
        PlatformDirectoryFact::Log => "log",
        PlatformDirectoryFact::Runtime => "runtime",
    }
}

const fn warning_id(value: StorageLayoutWarning) -> &'static str {
    match value {
        StorageLayoutWarning::UnifiedCacheLifecycle => "unified-cache-lifecycle",
        StorageLayoutWarning::UnifiedRuntimeLifecycle => "unified-runtime-lifecycle",
        StorageLayoutWarning::UnifiedBackupClassification => "unified-backup-classification",
        StorageLayoutWarning::PortableLifecycle => "portable-lifecycle",
    }
}

pub(super) const fn storage_class_id(value: StorageClass) -> &'static str {
    match value {
        StorageClass::Defaults => "defaults",
        StorageClass::Policy => "policy",
        StorageClass::UserConfig => "user-config",
        StorageClass::MachineState => "machine-state",
        StorageClass::WorkspaceLocal => "workspace-local",
        StorageClass::ProjectShared => "project-shared",
        StorageClass::Secret => "secret",
        StorageClass::Cache => "cache",
        StorageClass::Runtime => "runtime",
        StorageClass::Log => "log",
    }
}

fn transition_action_id(value: &StorageTransitionAction) -> String {
    match value {
        StorageTransitionAction::CopyOrdinary => "copyOrdinary".into(),
        StorageTransitionAction::Absent => "absent".into(),
        StorageTransitionAction::SameAuthority => "sameAuthority".into(),
        StorageTransitionAction::Identical => "identical".into(),
        StorageTransitionAction::CustomAdapter { .. } => "customAdapter".into(),
        StorageTransitionAction::Excluded(_) => "excluded".into(),
    }
}

fn evidence_digest(value: Option<&StorageFileEvidence>) -> Option<String> {
    match value {
        Some(StorageFileEvidence::Present { sha256, .. })
        | Some(StorageFileEvidence::Semantic { sha256 }) => Some(sha256.as_str().into()),
        Some(StorageFileEvidence::Absent) | None => None,
    }
}

const fn conflict_kind_id(value: StorageTransitionConflictKind) -> &'static str {
    match value {
        StorageTransitionConflictKind::OverlappingRoots => "overlappingRoots",
        StorageTransitionConflictKind::TargetOccupied => "targetOccupied",
        StorageTransitionConflictKind::UnknownTargetFile => "unknownTargetFile",
    }
}

const fn transition_outcome_id(value: StorageTransitionOutcome) -> &'static str {
    match value {
        StorageTransitionOutcome::TargetCommitted => "targetCommitted",
        StorageTransitionOutcome::SourceRetained => "sourceRetained",
    }
}
