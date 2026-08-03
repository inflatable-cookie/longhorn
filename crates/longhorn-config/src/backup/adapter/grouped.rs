use crate::{BackupLimits, DomainDescriptor, Sha256Digest};

use super::{
    BackupAdapterError, BackupAdapterInspectRequest, BackupAdapterPayload,
    BackupAdapterRestorePreview,
};

/// Complete side-effect-free stage for one grouped custom restore target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterRestoreStage {
    target_payloads: Vec<BackupAdapterPayload>,
    rollback_payloads: Vec<BackupAdapterPayload>,
    target_evidence: Sha256Digest,
    current_evidence: Option<Sha256Digest>,
}

impl BackupAdapterRestoreStage {
    /// Constructs one bounded-stage candidate.
    ///
    /// Longhorn validates payload paths, uniqueness, byte bounds, and evidence
    /// against the inspected preview before durable publication.
    #[must_use]
    pub fn new(
        target_payloads: Vec<BackupAdapterPayload>,
        rollback_payloads: Vec<BackupAdapterPayload>,
        target_evidence: Sha256Digest,
        current_evidence: Option<Sha256Digest>,
    ) -> Self {
        Self {
            target_payloads,
            rollback_payloads,
            target_evidence,
            current_evidence,
        }
    }

    /// Returns the adapter-prepared target payloads.
    #[must_use]
    pub fn target_payloads(&self) -> &[BackupAdapterPayload] {
        &self.target_payloads
    }

    /// Returns the exact prior-state payloads used for rollback.
    #[must_use]
    pub fn rollback_payloads(&self) -> &[BackupAdapterPayload] {
        &self.rollback_payloads
    }

    /// Returns the staged target semantic evidence.
    #[must_use]
    pub const fn target_evidence(&self) -> &Sha256Digest {
        &self.target_evidence
    }

    /// Returns the staged current semantic evidence; `None` means absent.
    #[must_use]
    pub const fn current_evidence(&self) -> Option<&Sha256Digest> {
        self.current_evidence.as_ref()
    }
}

/// Side-effect-free request to stage one member of a grouped restore.
#[derive(Clone, Debug)]
pub struct BackupAdapterGroupedStageRequest<'request> {
    inspect: BackupAdapterInspectRequest<'request>,
    preview: &'request BackupAdapterRestorePreview,
    limits: BackupLimits,
}

impl<'request> BackupAdapterGroupedStageRequest<'request> {
    pub(crate) const fn new(
        inspect: BackupAdapterInspectRequest<'request>,
        preview: &'request BackupAdapterRestorePreview,
        limits: BackupLimits,
    ) -> Self {
        Self {
            inspect,
            preview,
            limits,
        }
    }

    /// Returns the verified archive request.
    #[must_use]
    pub const fn inspect(&self) -> &BackupAdapterInspectRequest<'request> {
        &self.inspect
    }

    /// Returns confirmation-bound semantic evidence.
    #[must_use]
    pub const fn preview(&self) -> &BackupAdapterRestorePreview {
        self.preview
    }

    /// Returns complete per-domain and total stage bounds.
    #[must_use]
    pub const fn limits(&self) -> BackupLimits {
        self.limits
    }
}

/// Whether one grouped apply publishes the target or exact prior state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupAdapterGroupedApplyKind {
    /// Publish the staged archive target.
    Target,
    /// Publish the staged exact prior state.
    Rollback,
}

/// Request to apply one already-durable grouped stage payload set.
#[derive(Clone, Copy, Debug)]
pub struct BackupAdapterGroupedApplyRequest<'request> {
    descriptor: &'request DomainDescriptor,
    kind: BackupAdapterGroupedApplyKind,
    payloads: &'request [BackupAdapterPayload],
    expected_evidence: Option<&'request Sha256Digest>,
}

impl<'request> BackupAdapterGroupedApplyRequest<'request> {
    pub(crate) const fn new(
        descriptor: &'request DomainDescriptor,
        kind: BackupAdapterGroupedApplyKind,
        payloads: &'request [BackupAdapterPayload],
        expected_evidence: Option<&'request Sha256Digest>,
    ) -> Self {
        Self {
            descriptor,
            kind,
            payloads,
            expected_evidence,
        }
    }

    /// Returns the schema-opaque registered descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        self.descriptor
    }

    /// Returns whether this applies target or rollback state.
    #[must_use]
    pub const fn kind(&self) -> BackupAdapterGroupedApplyKind {
        self.kind
    }

    /// Returns the verified durable payloads.
    #[must_use]
    pub const fn payloads(&self) -> &[BackupAdapterPayload] {
        self.payloads
    }

    /// Returns the semantic evidence Longhorn will verify after apply.
    #[must_use]
    pub const fn expected_evidence(&self) -> Option<&Sha256Digest> {
        self.expected_evidence
    }
}

/// Side-effect-free request to observe one live grouped restore target.
#[derive(Clone, Copy, Debug)]
pub struct BackupAdapterGroupedVerifyRequest<'request> {
    descriptor: &'request DomainDescriptor,
}

impl<'request> BackupAdapterGroupedVerifyRequest<'request> {
    pub(crate) const fn new(descriptor: &'request DomainDescriptor) -> Self {
        Self { descriptor }
    }

    /// Returns the schema-opaque registered descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        self.descriptor
    }
}

/// Object-safe grouped transaction extension for one custom adapter.
///
/// `stage` must not mutate live authority. `apply` publishes only the supplied
/// durable payload set. `verify` independently observes current semantic
/// evidence; `None` means the domain is absent.
pub trait BackupAdapterGroupedRestore {
    /// Stages exact target and rollback payloads without live mutation.
    fn stage(
        &self,
        request: BackupAdapterGroupedStageRequest<'_>,
    ) -> Result<BackupAdapterRestoreStage, BackupAdapterError>;

    /// Applies one target or rollback payload set.
    fn apply(
        &self,
        request: BackupAdapterGroupedApplyRequest<'_>,
    ) -> Result<(), BackupAdapterError>;

    /// Observes exact current semantic evidence without mutation.
    fn verify(
        &self,
        request: BackupAdapterGroupedVerifyRequest<'_>,
    ) -> Result<Option<Sha256Digest>, BackupAdapterError>;
}
