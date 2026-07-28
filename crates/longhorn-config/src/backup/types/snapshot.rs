use longhorn_core::DomainId;

use crate::BackupAdapterId;

use super::{BackupConsistencyMode, BackupManifest, BackupPayloadPath, Sha256Digest};

/// Immutable exact bytes for one captured manifest payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupSnapshotPayload {
    domain: DomainId,
    path: BackupPayloadPath,
    bytes: Vec<u8>,
}

impl BackupSnapshotPayload {
    pub(crate) fn new(domain: DomainId, path: BackupPayloadPath, bytes: Vec<u8>) -> Self {
        Self {
            domain,
            path,
            bytes,
        }
    }

    /// Returns the source domain.
    #[must_use]
    pub fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the manifest payload path.
    #[must_use]
    pub fn path(&self) -> &BackupPayloadPath {
        &self.path
    }

    /// Returns the exact immutable source bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Machine-readable receipt for a completed coordinated snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupCaptureReceipt {
    selected_domains: usize,
    captured_domains: usize,
    absent_domains: usize,
    source_preserved_domains: usize,
    excluded_domains: usize,
    custom_domains: usize,
    external_consistency_groups: usize,
    total_payload_bytes: u64,
}

impl BackupCaptureReceipt {
    pub(crate) fn new(
        selected_domains: usize,
        captured_domains: usize,
        absent_domains: usize,
        source_preserved_domains: usize,
        excluded_domains: usize,
        total_payload_bytes: usize,
    ) -> Self {
        Self {
            selected_domains,
            captured_domains,
            absent_domains,
            source_preserved_domains,
            excluded_domains,
            custom_domains: 0,
            external_consistency_groups: 0,
            total_payload_bytes: total_payload_bytes as u64,
        }
    }

    pub(crate) const fn with_adapters(
        mut self,
        custom_domains: usize,
        external_consistency_groups: usize,
    ) -> Self {
        self.custom_domains = custom_domains;
        self.external_consistency_groups = external_consistency_groups;
        self
    }

    /// Returns domains considered by the explicit scope.
    #[must_use]
    pub const fn selected_domains(&self) -> usize {
        self.selected_domains
    }

    /// Returns domains with exact captured source bytes.
    #[must_use]
    pub const fn captured_domains(&self) -> usize {
        self.captured_domains
    }

    /// Returns selected domains recorded as absent.
    #[must_use]
    pub const fn absent_domains(&self) -> usize {
        self.absent_domains
    }

    /// Returns selected domains preserved as non-restorable source.
    #[must_use]
    pub const fn source_preserved_domains(&self) -> usize {
        self.source_preserved_domains
    }

    /// Returns selected domains explicitly excluded.
    #[must_use]
    pub const fn excluded_domains(&self) -> usize {
        self.excluded_domains
    }

    /// Returns domains captured by consumer adapters.
    #[must_use]
    pub const fn custom_domains(&self) -> usize {
        self.custom_domains
    }

    /// Returns independently consistent external groups.
    #[must_use]
    pub const fn external_consistency_groups(&self) -> usize {
        self.external_consistency_groups
    }

    /// Returns exact retained payload bytes.
    #[must_use]
    pub const fn total_payload_bytes(&self) -> u64 {
        self.total_payload_bytes
    }
}

/// Verified evidence for one completed custom adapter capture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterCaptureReceipt {
    domain: DomainId,
    adapter: BackupAdapterId,
    consistency_group: String,
    consistency_mode: BackupConsistencyMode,
    payloads: usize,
    total_payload_bytes: u64,
    payload_set_sha256: Sha256Digest,
}

impl BackupAdapterCaptureReceipt {
    pub(crate) fn new(
        domain: DomainId,
        adapter: BackupAdapterId,
        consistency_group: String,
        consistency_mode: BackupConsistencyMode,
        payloads: usize,
        total_payload_bytes: usize,
        payload_set_sha256: Sha256Digest,
    ) -> Self {
        Self {
            domain,
            adapter,
            consistency_group,
            consistency_mode,
            payloads,
            total_payload_bytes: total_payload_bytes as u64,
            payload_set_sha256,
        }
    }

    /// Returns the custom domain.
    #[must_use]
    pub const fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the stable adapter id.
    #[must_use]
    pub const fn adapter(&self) -> &BackupAdapterId {
        &self.adapter
    }

    /// Returns the manifest consistency group.
    #[must_use]
    pub fn consistency_group(&self) -> &str {
        &self.consistency_group
    }

    /// Returns the declared consistency mode.
    #[must_use]
    pub const fn consistency_mode(&self) -> BackupConsistencyMode {
        self.consistency_mode
    }

    /// Returns captured adapter payload count.
    #[must_use]
    pub const fn payloads(&self) -> usize {
        self.payloads
    }

    /// Returns captured adapter payload bytes.
    #[must_use]
    pub const fn total_payload_bytes(&self) -> u64 {
        self.total_payload_bytes
    }

    /// Returns a canonical digest over ordered path, length, and payload digest evidence.
    #[must_use]
    pub const fn payload_set_sha256(&self) -> &Sha256Digest {
        &self.payload_set_sha256
    }
}

/// Immutable coordinated backup snapshot ready for later archive encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupSnapshot {
    manifest: BackupManifest,
    payloads: Vec<BackupSnapshotPayload>,
    adapter_receipts: Vec<BackupAdapterCaptureReceipt>,
    receipt: BackupCaptureReceipt,
}

impl BackupSnapshot {
    pub(crate) fn new(
        manifest: BackupManifest,
        payloads: Vec<BackupSnapshotPayload>,
        adapter_receipts: Vec<BackupAdapterCaptureReceipt>,
        receipt: BackupCaptureReceipt,
    ) -> Self {
        Self {
            manifest,
            payloads,
            adapter_receipts,
            receipt,
        }
    }

    /// Returns the strict manifest model.
    #[must_use]
    pub fn manifest(&self) -> &BackupManifest {
        &self.manifest
    }

    /// Returns immutable payloads in stable domain-id order.
    #[must_use]
    pub fn payloads(&self) -> &[BackupSnapshotPayload] {
        &self.payloads
    }

    /// Returns custom adapter receipts in stable domain order.
    #[must_use]
    pub fn adapter_receipts(&self) -> &[BackupAdapterCaptureReceipt] {
        &self.adapter_receipts
    }

    /// Returns the completed capture receipt.
    #[must_use]
    pub fn receipt(&self) -> &BackupCaptureReceipt {
        &self.receipt
    }
}
