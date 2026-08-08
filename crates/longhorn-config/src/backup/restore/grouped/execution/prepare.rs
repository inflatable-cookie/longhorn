//! Stage and reinspect preparation for grouped restore.

use crate::backup::restore::{
    RestoreFailureTerminal, RestoreInspection, inspection::payloads_for_adapter,
};
use crate::{
    BackupAdapterGroupedStageRequest, BackupAdapterInspectRequest,
    BackupAdapterRestoreParticipation, BackupArchiveInspection, BackupCatalog, ConfigStore,
    backup::CatalogDecision,
};

use super::super::{
    journal::{self, PreparedGroupedDomain},
    types::{
        RestoreAdapterGroupError, RestoreAdapterGroupExecutionOptions,
        RestoreAdapterGroupExecutionStage, RestoreAdapterGroupPlan,
    },
};
use super::{failure, stage_limit_failure, validate_state_payload_set};

pub(crate) fn prepare_domains(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    archive: &BackupArchiveInspection,
    inspection: &RestoreInspection,
    plan: &RestoreAdapterGroupPlan,
    options: &RestoreAdapterGroupExecutionOptions,
) -> Result<(Vec<PreparedGroupedDomain>, usize), RestoreAdapterGroupError> {
    let mut prepared_domains = Vec::with_capacity(plan.entries.len());
    let mut staged_total = 0usize;
    for planned in &plan.entries {
        let descriptor = store
            .registered_descriptor(&planned.domain)
            .ok_or_else(|| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore domain is no longer registered",
                )
            })?;
        let source = inspection
            .manifest
            .domains()
            .iter()
            .find(|source| source.domain() == &planned.domain)
            .ok_or_else(|| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore source disappeared after inspection",
                )
            })?;
        let inspected = inspection
            .custom_prepared
            .get(&planned.domain)
            .ok_or_else(|| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore target disappeared after inspection",
                )
            })?;
        let adapter = match catalog.decision(descriptor) {
            Some(CatalogDecision::Custom(adapter))
                if adapter.id() == &planned.adapter
                    && source.adapter() == adapter.id().as_str()
                    && adapter.capabilities().restore()
                        == &BackupAdapterRestoreParticipation::GroupedFailureAtomic
                    && inspected.adapter == planned.adapter
                    && inspected.participation
                        == BackupAdapterRestoreParticipation::GroupedFailureAtomic
                    && inspected.confirmation_digest == planned.adapter_confirmation =>
            {
                adapter
            }
            _ => {
                return Err(failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore descriptor or adapter changed",
                ));
            }
        };
        let grouped = adapter.grouped_restore().ok_or_else(|| {
            failure(
                RestoreAdapterGroupExecutionStage::Reinspect,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "adapter declares grouped participation without grouped protocol",
            )
        })?;

        let payloads = payloads_for_adapter(archive, source).ok_or_else(|| {
            failure(
                RestoreAdapterGroupExecutionStage::Reinspect,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "verified archive payload is unavailable",
            )
        })?;
        let fresh_preview = adapter
            .inspect(BackupAdapterInspectRequest::new(
                descriptor,
                source.state(),
                source.source_schema_version(),
                payloads,
            ))
            .map_err(|error| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    error,
                )
            })?;
        if fresh_preview.target_evidence() != &planned.target_evidence
            || fresh_preview.current_evidence() != &planned.rollback_evidence
            || fresh_preview != inspected.preview
        {
            return Err(failure(
                RestoreAdapterGroupExecutionStage::Reinspect,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "grouped restore semantic preview changed",
            ));
        }

        let payloads = payloads_for_adapter(archive, source).ok_or_else(|| {
            failure(
                RestoreAdapterGroupExecutionStage::Stage,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "verified archive payload is unavailable",
            )
        })?;
        let stage = grouped
            .stage(BackupAdapterGroupedStageRequest::new(
                BackupAdapterInspectRequest::new(
                    descriptor,
                    source.state(),
                    source.source_schema_version(),
                    payloads,
                ),
                &fresh_preview,
                options.limits,
            ))
            .map_err(|error| {
                failure(
                    RestoreAdapterGroupExecutionStage::Stage,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    error,
                )
            })?;
        if stage.target_evidence() != &planned.target_evidence
            || stage.rollback_evidence() != &planned.rollback_evidence
        {
            return Err(failure(
                RestoreAdapterGroupExecutionStage::Stage,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "grouped adapter stage evidence contradicts the confirmed preview",
            ));
        }
        let domain_bytes = validate_state_payload_set(
            &planned.domain,
            stage.target_evidence(),
            stage.target_payloads(),
        )?
        .checked_add(validate_state_payload_set(
            &planned.domain,
            stage.rollback_evidence(),
            stage.rollback_payloads(),
        )?)
        .ok_or_else(|| stage_limit_failure(&planned.domain, "stage byte length overflow"))?;
        if domain_bytes > options.limits.max_domain_bytes() {
            return Err(stage_limit_failure(
                &planned.domain,
                format!(
                    "stage bytes {domain_bytes} exceed domain limit {}",
                    options.limits.max_domain_bytes()
                ),
            ));
        }
        staged_total = staged_total
            .checked_add(domain_bytes)
            .ok_or_else(|| stage_limit_failure(&planned.domain, "total stage length overflow"))?;
        if staged_total > options.limits.max_total_bytes() {
            return Err(stage_limit_failure(
                &planned.domain,
                format!(
                    "stage bytes {staged_total} exceed total limit {}",
                    options.limits.max_total_bytes()
                ),
            ));
        }
        prepared_domains.push(PreparedGroupedDomain {
            domain: planned.domain.clone(),
            adapter: planned.adapter.clone(),
            descriptor_digest: journal::descriptor_digest(descriptor),
            target_evidence: planned.target_evidence.clone(),
            rollback_evidence: planned.rollback_evidence.clone(),
            target_payloads: stage.target_payloads().to_vec(),
            rollback_payloads: stage.rollback_payloads().to_vec(),
        });
    }

    Ok((prepared_domains, staged_total))
}
