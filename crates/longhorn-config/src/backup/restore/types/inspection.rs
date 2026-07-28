use std::collections::BTreeMap;

use longhorn_core::{DomainId, SchemaVersion};

use crate::{
    BackupAdapterId, BackupAdapterRestoreParticipation, BackupAdapterRestorePreview,
    BackupApplication, BackupExclusion, BackupManifest, BackupProducer, BackupSourceIssue,
    BackupSourceState, DomainLocation, Sha256Digest,
};

/// Compatibility of one archive identity field with the restore target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreIdentityStatus {
    /// Stable identity matches. Version differences remain visible in the manifest.
    Compatible,
    /// Stable identity names another application or producer.
    Mismatch {
        /// Stable identity required by the target.
        expected: String,
        /// Stable identity declared by the archive.
        archive: String,
    },
}

/// Application and producer compatibility report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreIdentityInspection {
    application: RestoreIdentityStatus,
    producer: RestoreIdentityStatus,
}

impl RestoreIdentityInspection {
    pub(crate) fn inspect(
        manifest: &BackupManifest,
        application: &BackupApplication,
        producer: &BackupProducer,
    ) -> Self {
        Self {
            application: identity_status(application.id(), manifest.application().id()),
            producer: identity_status(producer.name(), manifest.producer().name()),
        }
    }

    /// Returns application-id compatibility.
    #[must_use]
    pub fn application(&self) -> &RestoreIdentityStatus {
        &self.application
    }

    /// Returns producer-name compatibility.
    #[must_use]
    pub fn producer(&self) -> &RestoreIdentityStatus {
        &self.producer
    }

    /// Returns whether both stable identities match.
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        matches!(self.application, RestoreIdentityStatus::Compatible)
            && matches!(self.producer, RestoreIdentityStatus::Compatible)
    }
}

fn identity_status(expected: &str, archive: &str) -> RestoreIdentityStatus {
    if expected == archive {
        RestoreIdentityStatus::Compatible
    } else {
        RestoreIdentityStatus::Mismatch {
            expected: expected.to_owned(),
            archive: archive.to_owned(),
        }
    }
}

/// Restore compatibility of one manifest domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreDomainCompatibility {
    /// Source already uses the registered current schema.
    Ready,
    /// Source can be migrated completely in memory.
    MigrationRequired {
        /// Archive source schema.
        from: SchemaVersion,
        /// Registered target schema.
        to: SchemaVersion,
    },
    /// Archive domain is not registered in the target store.
    UnknownDomain,
    /// Registered storage metadata differs from the manifest.
    DescriptorMismatch,
    /// Registered schema code was not supplied in the catalogue.
    DomainCodeUnavailable,
    /// Target policy now excludes this domain.
    PolicyExcluded {
        /// Stable target exclusion reason.
        reason: String,
    },
    /// Target policy requires a consumer restore adapter.
    CustomAdapterUnavailable {
        /// Stable adapter id.
        adapter: String,
    },
    /// Consumer adapter verified this source for an explicit custom operation.
    CustomAdapterReady {
        /// Stable adapter id.
        adapter: BackupAdapterId,
        /// Adapter-owned transaction guarantee.
        participation: BackupAdapterRestoreParticipation,
        /// Digest required for explicit execution.
        confirmation_digest: Sha256Digest,
    },
    /// Consumer adapter rejected otherwise verified archive payloads.
    CustomAdapterRejected {
        /// Stable adapter id.
        adapter: BackupAdapterId,
        /// Stable adapter failure.
        detail: String,
    },
    /// Registered storage cannot participate in ordinary restore.
    TargetUnavailable {
        /// Resolved unavailable or non-writable authority.
        location: DomainLocation,
    },
    /// Capture deliberately preserved invalid source evidence.
    SourcePreserved {
        /// Manifest-declared source issue.
        issue: BackupSourceIssue,
    },
    /// A nominally present source failed current target inspection.
    SourceRejected {
        /// Typed source or migration issue.
        issue: BackupSourceIssue,
    },
    /// Source loaded but a current-schema target could not be encoded or validated.
    TargetPreparationFailed {
        /// Consumer-code or serialization detail.
        detail: String,
    },
}

impl RestoreDomainCompatibility {
    pub(crate) fn is_restorable(&self) -> bool {
        matches!(self, Self::Ready | Self::MigrationRequired { .. })
    }
}

/// Non-mutating report for one included manifest domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreDomainInspection {
    domain: DomainId,
    source_state: BackupSourceState,
    source_schema_version: Option<SchemaVersion>,
    target_schema_version: Option<SchemaVersion>,
    compatibility: RestoreDomainCompatibility,
}

impl RestoreDomainInspection {
    pub(crate) fn new(
        domain: DomainId,
        source_state: BackupSourceState,
        source_schema_version: Option<SchemaVersion>,
        target_schema_version: Option<SchemaVersion>,
        compatibility: RestoreDomainCompatibility,
    ) -> Self {
        Self {
            domain,
            source_state,
            source_schema_version,
            target_schema_version,
            compatibility,
        }
    }

    /// Returns the manifest domain.
    #[must_use]
    pub fn domain(&self) -> &DomainId {
        &self.domain
    }

    /// Returns the exact source state declared by the manifest.
    #[must_use]
    pub const fn source_state(&self) -> BackupSourceState {
        self.source_state
    }

    /// Returns the archive source schema, when readable.
    #[must_use]
    pub const fn source_schema_version(&self) -> Option<SchemaVersion> {
        self.source_schema_version
    }

    /// Returns the registered target schema, when known.
    #[must_use]
    pub const fn target_schema_version(&self) -> Option<SchemaVersion> {
        self.target_schema_version
    }

    /// Returns typed source-to-target compatibility.
    #[must_use]
    pub fn compatibility(&self) -> &RestoreDomainCompatibility {
        &self.compatibility
    }
}

/// Report for one manifest exclusion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreExclusionInspection {
    exclusion: BackupExclusion,
    registered: bool,
}

impl RestoreExclusionInspection {
    pub(crate) fn new(exclusion: BackupExclusion, registered: bool) -> Self {
        Self {
            exclusion,
            registered,
        }
    }

    /// Returns the exact manifest exclusion.
    #[must_use]
    pub fn exclusion(&self) -> &BackupExclusion {
        &self.exclusion
    }

    /// Returns whether the excluded domain is registered in the target.
    #[must_use]
    pub const fn is_registered(&self) -> bool {
        self.registered
    }
}

/// Side-effect-free restore compatibility report over a verified archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreInspection {
    pub(crate) manifest: BackupManifest,
    pub(crate) archive_sha256: Sha256Digest,
    pub(crate) identity: RestoreIdentityInspection,
    pub(crate) domains: Vec<RestoreDomainInspection>,
    pub(crate) exclusions: Vec<RestoreExclusionInspection>,
    pub(crate) prepared: BTreeMap<DomainId, PreparedTarget>,
    pub(crate) custom_prepared: BTreeMap<DomainId, PreparedAdapterTarget>,
    pub(crate) receipt: RestoreInspectionReceipt,
}

impl RestoreInspection {
    /// Returns the verified archive manifest.
    #[must_use]
    pub fn manifest(&self) -> &BackupManifest {
        &self.manifest
    }

    /// Returns the complete source archive digest.
    #[must_use]
    pub fn archive_sha256(&self) -> &Sha256Digest {
        &self.archive_sha256
    }

    /// Returns application and producer compatibility.
    #[must_use]
    pub fn identity(&self) -> &RestoreIdentityInspection {
        &self.identity
    }

    /// Returns every included manifest domain in stable order.
    #[must_use]
    pub fn domains(&self) -> &[RestoreDomainInspection] {
        &self.domains
    }

    /// Returns every manifest exclusion in stable order.
    #[must_use]
    pub fn exclusions(&self) -> &[RestoreExclusionInspection] {
        &self.exclusions
    }

    /// Returns complete inspection counts.
    #[must_use]
    pub fn receipt(&self) -> &RestoreInspectionReceipt {
        &self.receipt
    }

    /// Returns the confirmation digest for one adapter-ready domain.
    #[must_use]
    pub fn adapter_confirmation(&self, domain: &DomainId) -> Option<&Sha256Digest> {
        self.custom_prepared
            .get(domain)
            .map(|prepared| &prepared.confirmation_digest)
    }
}

/// Machine-readable counts for a complete restore inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreInspectionReceipt {
    pub(crate) manifest_domains: usize,
    pub(crate) exclusions: usize,
    pub(crate) restorable: usize,
    pub(crate) migrations: usize,
    pub(crate) adapter_restorable: usize,
    pub(crate) blocked: usize,
}

impl RestoreInspectionReceipt {
    /// Returns included manifest domains inspected exactly once.
    #[must_use]
    pub const fn manifest_domains(&self) -> usize {
        self.manifest_domains
    }

    /// Returns manifest exclusions reported exactly once.
    #[must_use]
    pub const fn exclusions(&self) -> usize {
        self.exclusions
    }

    /// Returns domains whose archive state can be selected.
    #[must_use]
    pub const fn restorable(&self) -> usize {
        self.restorable
    }

    /// Returns restorable domains requiring in-memory migration.
    #[must_use]
    pub const fn migrations(&self) -> usize {
        self.migrations
    }

    /// Returns domains ready for a separate explicit adapter operation.
    #[must_use]
    pub const fn adapter_restorable(&self) -> usize {
        self.adapter_restorable
    }

    /// Returns included domains blocked from selection.
    #[must_use]
    pub const fn blocked(&self) -> usize {
        self.blocked
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreparedTarget {
    pub(crate) bytes: Option<Vec<u8>>,
    pub(crate) schema_version: Option<SchemaVersion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreparedAdapterTarget {
    pub(crate) adapter: BackupAdapterId,
    pub(crate) participation: BackupAdapterRestoreParticipation,
    pub(crate) preview: BackupAdapterRestorePreview,
    pub(crate) confirmation_digest: Sha256Digest,
}
