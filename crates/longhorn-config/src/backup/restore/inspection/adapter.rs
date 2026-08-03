use longhorn_core::SchemaVersion;
use serde_json::json;

use crate::{
    BackupAdapter, BackupAdapterInspectRequest, BackupAdapterRestoreParticipation,
    BackupArchiveInspection, BackupManifestDomain, DomainDescriptor, Sha256Digest,
};

use super::{
    PreparedAdapterTarget, PreparedTarget, RestoreDomainCompatibility, RestoreDomainInspection,
    domain_report, payloads_for_adapter,
};

pub(super) fn inspect_custom_domain(
    archive: &BackupArchiveInspection,
    source: &BackupManifestDomain,
    descriptor: &DomainDescriptor,
    target_schema: Option<SchemaVersion>,
    adapter: &dyn BackupAdapter,
) -> (
    RestoreDomainInspection,
    Option<PreparedTarget>,
    Option<PreparedAdapterTarget>,
) {
    if source.adapter() != adapter.id().as_str() {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::CustomAdapterUnavailable {
                    adapter: source.adapter().into(),
                },
            ),
            None,
            None,
        );
    }
    if source.state() == crate::BackupSourceState::SourcePreserved {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::SourcePreserved {
                    issue: source
                        .source_issue()
                        .expect("verified source-preserved manifest declares an issue"),
                },
            ),
            None,
            None,
        );
    }
    let participation = adapter.capabilities().restore().clone();
    if let BackupAdapterRestoreParticipation::Excluded(reason) = &participation {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::PolicyExcluded {
                    reason: reason.as_str().into(),
                },
            ),
            None,
            None,
        );
    }
    let Some(payloads) = payloads_for_adapter(archive, source) else {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::CustomAdapterRejected {
                    adapter: adapter.id().clone(),
                    detail: "verified archive payload is unavailable".into(),
                },
            ),
            None,
            None,
        );
    };
    let request =
        BackupAdapterInspectRequest::new(descriptor, source.source_schema_version(), payloads);
    match adapter.inspect(request) {
        Ok(preview) => {
            let confirmation_digest = adapter_confirmation_digest(
                archive.archive_sha256(),
                source,
                adapter,
                &participation,
                preview.target_evidence(),
                preview.current_evidence(),
            );
            (
                domain_report(
                    source,
                    target_schema,
                    RestoreDomainCompatibility::CustomAdapterReady {
                        adapter: adapter.id().clone(),
                        participation: participation.clone(),
                        confirmation_digest: confirmation_digest.clone(),
                    },
                ),
                None,
                Some(PreparedAdapterTarget {
                    adapter: adapter.id().clone(),
                    participation,
                    preview,
                    confirmation_digest,
                }),
            )
        }
        Err(error) => (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::CustomAdapterRejected {
                    adapter: adapter.id().clone(),
                    detail: error.to_string(),
                },
            ),
            None,
            None,
        ),
    }
}

fn adapter_confirmation_digest(
    archive_sha256: &Sha256Digest,
    source: &BackupManifestDomain,
    adapter: &dyn BackupAdapter,
    participation: &BackupAdapterRestoreParticipation,
    target_evidence: &Sha256Digest,
    current_evidence: Option<&Sha256Digest>,
) -> Sha256Digest {
    let canonical = serde_json::to_vec(&json!({
        "archiveSha256": archive_sha256.as_str(),
        "domain": source.domain().as_str(),
        "adapter": adapter.id().as_str(),
        "participation": participation_name(participation),
        "targetEvidence": target_evidence.as_str(),
        "currentEvidence": current_evidence.map(Sha256Digest::as_str),
    }))
    .expect("adapter confirmation form is serializable");
    Sha256Digest::from_bytes(&canonical)
}

const fn participation_name(participation: &BackupAdapterRestoreParticipation) -> &'static str {
    match participation {
        BackupAdapterRestoreParticipation::Excluded(_) => "excluded",
        BackupAdapterRestoreParticipation::Separate => "separate",
        BackupAdapterRestoreParticipation::FailureAtomic => "failure-atomic",
        BackupAdapterRestoreParticipation::GroupedFailureAtomic => "grouped-failure-atomic",
    }
}
