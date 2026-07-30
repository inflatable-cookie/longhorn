use crate::{
    BackupCaptureReceipt, BackupDestinationKind, BackupKind, BackupOperationalListing,
    BackupPublicationReceipt, BackupRetentionDiagnostic, BackupRetentionDiagnosticKind,
    BackupRetentionPlan, BackupRetentionReason, ConfigOperationProjectionError, Durability,
    Sha256Digest,
};

use super::super::{
    BackupArchiveProjection, BackupCaptureReceiptProjection, BackupInventoryEntry,
    BackupInventoryEntryState, BackupInventoryProjection, BackupPublicationReceiptProjection,
    BackupRetentionProjection, BackupRetentionReasonProjection,
};
use super::exact_path;

impl From<&BackupCaptureReceipt> for BackupCaptureReceiptProjection {
    fn from(value: &BackupCaptureReceipt) -> Self {
        Self {
            selected_domains: value.selected_domains(),
            captured_domains: value.captured_domains(),
            absent_domains: value.absent_domains(),
            source_preserved_domains: value.source_preserved_domains(),
            excluded_domains: value.excluded_domains(),
            custom_domains: value.custom_domains(),
            external_consistency_groups: value.external_consistency_groups(),
            total_payload_bytes: value.total_payload_bytes(),
            flushed_pending_publication: false,
        }
    }
}

impl BackupCaptureReceiptProjection {
    /// Marks that the injected authority durably flushed pending publication.
    #[must_use]
    pub const fn with_flushed_pending_publication(mut self) -> Self {
        self.flushed_pending_publication = true;
        self
    }
}

impl TryFrom<&BackupPublicationReceipt> for BackupPublicationReceiptProjection {
    type Error = ConfigOperationProjectionError;

    fn try_from(value: &BackupPublicationReceipt) -> Result<Self, Self::Error> {
        Ok(Self {
            path: exact_path(&value.path)?,
            destination: match value.destination {
                BackupDestinationKind::Operational => "operational",
                BackupDestinationKind::UserExport => "userExport",
            }
            .into(),
            archive_sha256: value.archive_sha256.as_str().into(),
            durability: match value.durability {
                Durability::FileSynced => "fileSynced",
                Durability::FileAndDirectorySynced => "fileAndDirectorySynced",
            }
            .into(),
            replaced_existing: value.replaced_existing,
        })
    }
}

impl TryFrom<&BackupOperationalListing> for BackupInventoryProjection {
    type Error = ConfigOperationProjectionError;

    fn try_from(value: &BackupOperationalListing) -> Result<Self, Self::Error> {
        Ok(Self {
            root: exact_path(value.root())?,
            archives: value
                .candidates()
                .iter()
                .map(|candidate| {
                    Ok(BackupArchiveProjection {
                        path: exact_path(candidate.path())?,
                        archive_id: candidate.archive_id().into(),
                        created_at: candidate.created_at().into(),
                        kind: backup_kind_id(candidate.kind()).into(),
                        archive_sha256: candidate.archive_sha256().as_str().into(),
                    })
                })
                .collect::<Result<_, ConfigOperationProjectionError>>()?,
            entries: value
                .diagnostics()
                .iter()
                .map(BackupInventoryEntry::try_from)
                .collect::<Result<_, _>>()?,
            complete: value.is_complete(),
        })
    }
}

impl TryFrom<&BackupRetentionDiagnostic> for BackupInventoryEntry {
    type Error = ConfigOperationProjectionError;

    fn try_from(value: &BackupRetentionDiagnostic) -> Result<Self, Self::Error> {
        Ok(Self {
            path: value.path.as_deref().map(exact_path).transpose()?,
            state: inventory_state(value.kind),
            diagnostic_kind: retention_diagnostic_id(value.kind).into(),
            detail: value.detail.clone(),
        })
    }
}

impl BackupRetentionProjection {
    /// Projects a host-owned plan with a separately issued confirmation digest.
    pub fn try_from_plan(
        plan: &BackupRetentionPlan,
        confirmation_digest: &Sha256Digest,
    ) -> Result<Self, ConfigOperationProjectionError> {
        Ok(Self {
            deletion_paths: plan
                .deletions()
                .iter()
                .map(|deletion| exact_path(&deletion.path))
                .collect::<Result<_, _>>()?,
            retained: plan
                .retained()
                .iter()
                .map(|(digest, reasons)| {
                    (
                        digest.as_str().into(),
                        reasons.iter().copied().map(Into::into).collect(),
                    )
                })
                .collect(),
            diagnostics: plan
                .diagnostics()
                .iter()
                .map(BackupInventoryEntry::try_from)
                .collect::<Result<_, _>>()?,
            confirmation_digest: confirmation_digest.as_str().into(),
        })
    }
}

impl From<BackupRetentionReason> for BackupRetentionReasonProjection {
    fn from(value: BackupRetentionReason) -> Self {
        match value {
            BackupRetentionReason::NewArchive => Self::NewArchive,
            BackupRetentionReason::Pinned => Self::Pinned,
            BackupRetentionReason::NewestCount => Self::NewestCount,
            BackupRetentionReason::Age => Self::Age,
            BackupRetentionReason::Milestone => Self::Milestone,
        }
    }
}

pub(super) const fn backup_kind_id(value: BackupKind) -> &'static str {
    match value {
        BackupKind::Operational => "operational",
        BackupKind::UserExport => "user-export",
        BackupKind::PreMigration => "pre-migration",
        BackupKind::PreRestore => "pre-restore",
    }
}

const fn inventory_state(value: BackupRetentionDiagnosticKind) -> BackupInventoryEntryState {
    match value {
        BackupRetentionDiagnosticKind::Locked => BackupInventoryEntryState::Locked,
        BackupRetentionDiagnosticKind::Corrupt => BackupInventoryEntryState::Corrupt,
        BackupRetentionDiagnosticKind::ForeignApplication => BackupInventoryEntryState::Foreign,
        BackupRetentionDiagnosticKind::UnknownFormat => BackupInventoryEntryState::Unknown,
        BackupRetentionDiagnosticKind::Unreadable
        | BackupRetentionDiagnosticKind::NonRegular
        | BackupRetentionDiagnosticKind::RootRead => BackupInventoryEntryState::Unreadable,
        BackupRetentionDiagnosticKind::Unmanaged
        | BackupRetentionDiagnosticKind::UserExport
        | BackupRetentionDiagnosticKind::DuplicateArchiveId
        | BackupRetentionDiagnosticKind::ScanLimit
        | BackupRetentionDiagnosticKind::ClockRegression
        | BackupRetentionDiagnosticKind::MissingPin
        | BackupRetentionDiagnosticKind::MissingNewArchive => BackupInventoryEntryState::Unmanaged,
    }
}

const fn retention_diagnostic_id(value: BackupRetentionDiagnosticKind) -> &'static str {
    match value {
        BackupRetentionDiagnosticKind::Unmanaged => "unmanaged",
        BackupRetentionDiagnosticKind::Locked => "locked",
        BackupRetentionDiagnosticKind::Unreadable => "unreadable",
        BackupRetentionDiagnosticKind::NonRegular => "nonRegular",
        BackupRetentionDiagnosticKind::Corrupt => "corrupt",
        BackupRetentionDiagnosticKind::UnknownFormat => "unknownFormat",
        BackupRetentionDiagnosticKind::ForeignApplication => "foreignApplication",
        BackupRetentionDiagnosticKind::UserExport => "userExport",
        BackupRetentionDiagnosticKind::DuplicateArchiveId => "duplicateArchiveId",
        BackupRetentionDiagnosticKind::ScanLimit => "scanLimit",
        BackupRetentionDiagnosticKind::RootRead => "rootRead",
        BackupRetentionDiagnosticKind::ClockRegression => "clockRegression",
        BackupRetentionDiagnosticKind::MissingPin => "missingPin",
        BackupRetentionDiagnosticKind::MissingNewArchive => "missingNewArchive",
    }
}
