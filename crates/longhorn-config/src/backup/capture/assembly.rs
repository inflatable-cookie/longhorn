use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::DomainId;
use serde_json::json;

use crate::{
    BackupAdapter, BackupAdapterCapture, BackupAdapterCaptureReceipt, BackupCaptureOptions,
    BackupConsistencyGroup, BackupConsistencyMode, BackupExclusion, BackupManifest,
    BackupManifestDomain, BackupMetadata, BackupPayloadManifest, BackupPayloadPath, BackupSnapshot,
    BackupSnapshotPayload, BackupSourceState, DomainDescriptor, Sha256Digest,
    backup::types::BackupCaptureReceipt,
};

use super::{BackupCaptureError, TotalLimitFailure, checked_total, total_limit_error};

const MAX_ADAPTER_PAYLOADS_PER_DOMAIN: usize = 256;

pub(super) struct DeferredAdapterCapture<'adapter> {
    pub(super) descriptor: DomainDescriptor,
    pub(super) adapter: &'adapter dyn BackupAdapter,
    pub(super) group: BackupConsistencyGroup,
}

pub(super) struct CaptureAssembly<'adapter> {
    selected_domains: usize,
    total_limit: usize,
    pub(super) manifest_domains: Vec<BackupManifestDomain>,
    pub(super) exclusions: Vec<BackupExclusion>,
    pub(super) payloads: Vec<BackupSnapshotPayload>,
    adapter_receipts: Vec<BackupAdapterCaptureReceipt>,
    groups: BTreeMap<String, BackupConsistencyGroup>,
    pub(super) deferred: Vec<DeferredAdapterCapture<'adapter>>,
    pub(super) total_payload_bytes: usize,
    pub(super) absent_domains: usize,
    pub(super) source_preserved_domains: usize,
}

impl<'adapter> CaptureAssembly<'adapter> {
    pub(super) fn new(selected_domains: usize, total_limit: usize) -> Self {
        Self {
            selected_domains,
            total_limit,
            manifest_domains: Vec::new(),
            exclusions: Vec::new(),
            payloads: Vec::new(),
            adapter_receipts: Vec::new(),
            groups: BTreeMap::new(),
            deferred: Vec::new(),
            total_payload_bytes: 0,
            absent_domains: 0,
            source_preserved_domains: 0,
        }
    }

    pub(super) fn add_group(
        &mut self,
        group: BackupConsistencyGroup,
    ) -> Result<(), BackupCaptureError> {
        match self.groups.get(group.id()) {
            Some(existing) if existing != &group => {
                Err(BackupCaptureError::ConsistencyGroupConflict {
                    group: group.id().into(),
                })
            }
            Some(_) => Ok(()),
            None => {
                self.groups.insert(group.id().into(), group);
                Ok(())
            }
        }
    }

    pub(super) fn add_adapter_capture(
        &mut self,
        descriptor: &DomainDescriptor,
        adapter: &dyn BackupAdapter,
        group: BackupConsistencyGroup,
        captured: BackupAdapterCapture,
        options: BackupCaptureOptions,
    ) -> Result<(), BackupCaptureError> {
        let mode = group.mode();
        match captured {
            BackupAdapterCapture::Absent => {
                self.absent_domains += 1;
                self.manifest_domains
                    .push(BackupManifestDomain::custom_absent(
                        descriptor,
                        group.id(),
                        adapter.id(),
                    ));
                self.adapter_receipts.push(BackupAdapterCaptureReceipt::new(
                    descriptor.id().clone(),
                    adapter.id().clone(),
                    group.id().into(),
                    mode,
                    0,
                    0,
                    Sha256Digest::from_bytes(b"[]"),
                ));
            }
            BackupAdapterCapture::Present {
                source_schema_version,
                mut payloads,
            } => {
                if payloads.is_empty() || payloads.len() > MAX_ADAPTER_PAYLOADS_PER_DOMAIN {
                    return Err(BackupCaptureError::InvalidAdapterCapture {
                        domain: descriptor.id().clone(),
                        adapter: adapter.id().as_str().into(),
                        detail: format!(
                            "present capture must contain 1 through {MAX_ADAPTER_PAYLOADS_PER_DOMAIN} payloads"
                        ),
                    });
                }
                payloads.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
                let mut relative_paths = BTreeSet::new();
                let mut manifests = Vec::with_capacity(payloads.len());
                let mut receipt_evidence = Vec::with_capacity(payloads.len());
                let mut domain_total = 0usize;
                for payload in payloads {
                    if !relative_paths.insert(payload.relative_path().as_str().to_owned()) {
                        return Err(BackupCaptureError::InvalidAdapterCapture {
                            domain: descriptor.id().clone(),
                            adapter: adapter.id().as_str().into(),
                            detail: format!(
                                "duplicate payload path {}",
                                payload.relative_path().as_str()
                            ),
                        });
                    }
                    domain_total = checked_total(
                        domain_total,
                        payload.bytes().len(),
                        options.limits.max_domain_bytes(),
                    )
                    .map_err(|failure| {
                        adapter_limit_error(
                            descriptor.id(),
                            adapter.id().as_str(),
                            options.limits.max_domain_bytes(),
                            failure,
                        )
                    })?;
                    self.total_payload_bytes = checked_total(
                        self.total_payload_bytes,
                        payload.bytes().len(),
                        self.total_limit,
                    )
                    .map_err(|failure| {
                        total_limit_error(descriptor.id(), self.total_limit, failure)
                    })?;
                    let path = BackupPayloadPath::adapter(descriptor.id(), payload.relative_path());
                    let manifest = BackupPayloadManifest::new(path.clone(), payload.bytes());
                    receipt_evidence.push(json!({
                        "path": path.as_str(),
                        "byteLength": manifest.byte_length(),
                        "sha256": manifest.sha256().as_str(),
                    }));
                    self.payloads.push(BackupSnapshotPayload::new(
                        descriptor.id().clone(),
                        path,
                        payload.bytes,
                    ));
                    manifests.push(manifest);
                }
                let canonical = serde_json::to_vec(&receipt_evidence)
                    .expect("adapter receipt evidence is serializable");
                self.manifest_domains
                    .push(BackupManifestDomain::custom_present(
                        descriptor,
                        group.id(),
                        adapter.id(),
                        source_schema_version,
                        manifests,
                    ));
                self.adapter_receipts.push(BackupAdapterCaptureReceipt::new(
                    descriptor.id().clone(),
                    adapter.id().clone(),
                    group.id().into(),
                    mode,
                    receipt_evidence.len(),
                    domain_total,
                    Sha256Digest::from_bytes(&canonical),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn finish(
        mut self,
        metadata: BackupMetadata,
    ) -> Result<BackupSnapshot, BackupCaptureError> {
        self.manifest_domains
            .sort_by(|left, right| left.domain().cmp(right.domain()));
        self.exclusions
            .sort_by(|left, right| left.domain().cmp(right.domain()));
        self.payloads
            .sort_by(|left, right| left.path().cmp(right.path()));
        self.adapter_receipts
            .sort_by(|left, right| left.domain().cmp(right.domain()));
        let consistency_groups = self.groups.into_values().collect::<Vec<_>>();
        let external_consistency_groups = consistency_groups
            .iter()
            .filter(|group| group.mode() == BackupConsistencyMode::ExternalSnapshot)
            .count();
        let captured_domains = self
            .manifest_domains
            .iter()
            .filter(|domain| domain.state() != BackupSourceState::Absent)
            .count();
        let receipt = BackupCaptureReceipt::new(
            self.selected_domains,
            captured_domains,
            self.absent_domains,
            self.source_preserved_domains,
            self.exclusions.len(),
            self.total_payload_bytes,
        )
        .with_adapters(self.adapter_receipts.len(), external_consistency_groups);
        let manifest = BackupManifest::new(
            metadata,
            consistency_groups,
            self.manifest_domains,
            self.exclusions,
        );
        Ok(BackupSnapshot::new(
            manifest,
            self.payloads,
            self.adapter_receipts,
            receipt,
        ))
    }
}

fn adapter_limit_error(
    domain: &DomainId,
    adapter: &str,
    limit: usize,
    failure: TotalLimitFailure,
) -> BackupCaptureError {
    match failure {
        TotalLimitFailure::Overflow => BackupCaptureError::InvalidAdapterCapture {
            domain: domain.clone(),
            adapter: adapter.into(),
            detail: "payload byte total overflowed".into(),
        },
        TotalLimitFailure::Exceeded { observed } => BackupCaptureError::DomainTooLarge {
            domain: domain.clone(),
            limit,
            observed,
        },
    }
}
