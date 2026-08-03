use serde::{Deserialize, Serialize};

use crate::{BackupLimits, DomainDescriptor, Sha256Digest};

use super::{
    BackupAdapterError, BackupAdapterInspectRequest, BackupAdapterPayload,
    BackupAdapterRestorePreview,
};

/// Exact semantic presence observed or expected for one adapter domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum BackupAdapterStateEvidence {
    /// No live domain state exists.
    Absent,
    /// Live domain state exists with this semantic digest.
    Present {
        /// Digest over the adapter-defined semantic state.
        sha256: Sha256Digest,
    },
}

impl BackupAdapterStateEvidence {
    /// Constructs present semantic evidence.
    #[must_use]
    pub const fn present(sha256: Sha256Digest) -> Self {
        Self::Present { sha256 }
    }

    /// Converts an optional observed digest into explicit presence evidence.
    #[must_use]
    pub fn from_optional(sha256: Option<Sha256Digest>) -> Self {
        match sha256 {
            Some(sha256) => Self::Present { sha256 },
            None => Self::Absent,
        }
    }

    /// Returns the present semantic digest, or `None` for exact absence.
    #[must_use]
    pub const fn sha256(&self) -> Option<&Sha256Digest> {
        match self {
            Self::Absent => None,
            Self::Present { sha256 } => Some(sha256),
        }
    }

    /// Returns whether this evidence proves exact absence.
    #[must_use]
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// Complete side-effect-free stage for one grouped custom restore target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterRestoreStage {
    target_payloads: Vec<BackupAdapterPayload>,
    rollback_payloads: Vec<BackupAdapterPayload>,
    target_evidence: BackupAdapterStateEvidence,
    rollback_evidence: BackupAdapterStateEvidence,
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
        target_evidence: BackupAdapterStateEvidence,
        rollback_evidence: BackupAdapterStateEvidence,
    ) -> Self {
        Self {
            target_payloads,
            rollback_payloads,
            target_evidence,
            rollback_evidence,
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
    pub const fn target_evidence(&self) -> &BackupAdapterStateEvidence {
        &self.target_evidence
    }

    /// Returns the staged exact rollback semantic state.
    #[must_use]
    pub const fn rollback_evidence(&self) -> &BackupAdapterStateEvidence {
        &self.rollback_evidence
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
    expected_evidence: &'request BackupAdapterStateEvidence,
}

impl<'request> BackupAdapterGroupedApplyRequest<'request> {
    pub(crate) const fn new(
        descriptor: &'request DomainDescriptor,
        kind: BackupAdapterGroupedApplyKind,
        payloads: &'request [BackupAdapterPayload],
        expected_evidence: &'request BackupAdapterStateEvidence,
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
    pub const fn expected_evidence(&self) -> &BackupAdapterStateEvidence {
        self.expected_evidence
    }
}

/// Side-effect-free request to observe one live grouped restore target.
#[derive(Clone, Copy, Debug)]
pub struct BackupAdapterGroupedVerifyRequest<'request> {
    descriptor: &'request DomainDescriptor,
    kind: BackupAdapterGroupedApplyKind,
    expected_evidence: &'request BackupAdapterStateEvidence,
}

impl<'request> BackupAdapterGroupedVerifyRequest<'request> {
    pub(crate) const fn new(
        descriptor: &'request DomainDescriptor,
        kind: BackupAdapterGroupedApplyKind,
        expected_evidence: &'request BackupAdapterStateEvidence,
    ) -> Self {
        Self {
            descriptor,
            kind,
            expected_evidence,
        }
    }

    /// Returns the schema-opaque registered descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        self.descriptor
    }

    /// Returns whether target or rollback state is being verified.
    #[must_use]
    pub const fn kind(&self) -> BackupAdapterGroupedApplyKind {
        self.kind
    }

    /// Returns the exact semantic state Longhorn will compare.
    #[must_use]
    pub const fn expected_evidence(&self) -> &BackupAdapterStateEvidence {
        self.expected_evidence
    }
}

/// Object-safe grouped transaction extension for one custom adapter.
///
/// `stage` must not mutate live authority. `apply` publishes only the supplied
/// durable payload set. `verify` independently observes explicit present or
/// absent semantic evidence.
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
    ) -> Result<BackupAdapterStateEvidence, BackupAdapterError>;
}
