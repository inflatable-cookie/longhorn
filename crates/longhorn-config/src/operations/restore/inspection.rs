use super::super::{ConfigGeneration, ConfigOperationRejection, ConfigProtocolVersion};
use longhorn_core::ConfigRequestId;
use serde::{Deserialize, Serialize};

/// Durable restore-journal state safe for renderer gating.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum RestoreOperationStateProjection {
    /// No restore journal blocks ordinary work.
    Inactive,
    /// A destructive operation or recoverable interrupted operation exists.
    Active,
    /// Rollback could not be verified and ordinary mutation remains blocked.
    RecoveryRequired,
}

/// Current restore authority projected in the config snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreOperationsProjection {
    /// Exact durable journal state.
    pub state: RestoreOperationStateProjection,
    /// Safety archive pinned by an unresolved journal, when readable.
    pub safety_backup_sha256: Option<String>,
}

/// Archive selection without renderer filesystem authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "source"
)]
pub enum RestoreArchiveSelection {
    /// Select an exact proven archive from operational inventory.
    Inventory {
        /// Digest over exact published archive bytes.
        archive_sha256: String,
    },
    /// Ask the injected host picker for an archive.
    HostPicker,
}

/// Integrity state established before restore inspection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum RestoreIntegrityProjection {
    /// Strict archive inventory and every payload checksum verified.
    Verified,
}

/// Authentication evidence remains distinct from byte integrity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum RestoreAuthenticityProjection {
    /// Plaintext archive has no authenticity claim.
    Unauthenticated,
    /// The injected encryption authority authenticated the envelope.
    Authenticated,
}

/// Application or producer identity compatibility.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum RestoreIdentityStatusProjection {
    /// Stable identity matches.
    Compatible,
    /// Stable identity differs.
    Mismatch {
        /// Stable identity required by the host.
        expected: String,
        /// Stable identity declared by the archive.
        archive: String,
    },
}

/// Application and producer compatibility report.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreIdentityProjection {
    /// Application-id compatibility.
    pub application: RestoreIdentityStatusProjection,
    /// Producer-name compatibility.
    pub producer: RestoreIdentityStatusProjection,
}

/// One independent archive consistency group.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreConsistencyGroupProjection {
    /// Stable group id.
    pub id: String,
    /// Coordinated-bounded or external-snapshot mode.
    pub mode: String,
    /// Declared transaction authority.
    pub authority: String,
}

/// Restore compatibility of one included archive domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum RestoreDomainCompatibilityProjection {
    /// Source already uses the current schema.
    Ready,
    /// Source can migrate completely in private staging.
    MigrationRequired {
        /// Archive schema.
        from: u32,
        /// Registered target schema.
        to: u32,
    },
    /// Archive domain is not registered.
    UnknownDomain,
    /// Registered descriptor differs from the archive.
    DescriptorMismatch,
    /// Consumer domain code is unavailable.
    DomainCodeUnavailable,
    /// Current product policy excludes restore.
    PolicyExcluded {
        /// Stable exclusion reason.
        reason: String,
    },
    /// Required custom adapter is unavailable.
    CustomAdapterUnavailable {
        /// Stable adapter id.
        adapter: String,
    },
    /// Custom adapter is ready for an explicit operation.
    CustomAdapterReady {
        /// Stable adapter id.
        adapter: String,
        /// Adapter transaction guarantee.
        participation: RestoreAdapterParticipationProjection,
        /// Digest required for this adapter operation.
        confirmation_digest: String,
    },
    /// Custom adapter rejected verified payloads.
    CustomAdapterRejected {
        /// Stable adapter id.
        adapter: String,
        /// Stable non-secret failure.
        detail: String,
    },
    /// Registered target cannot participate in ordinary restore.
    TargetUnavailable {
        /// Stable unavailable authority class.
        reason: String,
    },
    /// Archive deliberately preserved invalid source evidence.
    SourcePreserved {
        /// Stable source issue.
        issue: String,
    },
    /// Present source failed current target inspection.
    SourceRejected {
        /// Stable source issue.
        issue: String,
    },
    /// Current-schema target preparation failed.
    TargetPreparationFailed {
        /// Stable consumer preparation detail.
        detail: String,
    },
}

/// Custom-adapter transaction participation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum RestoreAdapterParticipationProjection {
    /// Restore is explicitly excluded.
    Excluded {
        /// Stable exclusion reason.
        reason: String,
    },
    /// Restore is a separate receipted operation.
    Separate,
    /// Adapter promises verified exact rollback.
    FailureAtomic,
    /// Adapter participates in a Longhorn-owned grouped transaction.
    GroupedFailureAtomic,
}

/// One included archive domain and its target compatibility.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreDomainInspectionProjection {
    /// Stable domain id.
    pub domain_id: String,
    /// Manifest storage class.
    pub storage_class: String,
    /// Manifest consistency group.
    pub consistency_group: String,
    /// Manifest adapter id.
    pub adapter: String,
    /// Present, absent, or source-preserved state.
    pub source_state: String,
    /// Archive source schema when readable.
    pub source_schema_version: Option<u32>,
    /// Registered target schema when known.
    pub target_schema_version: Option<u32>,
    /// Exact compatibility classification.
    pub compatibility: RestoreDomainCompatibilityProjection,
}

/// One manifest exclusion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreExclusionProjection {
    /// Excluded domain.
    pub domain_id: String,
    /// Manifest storage class.
    pub storage_class: String,
    /// Stable manifest reason.
    pub reason: String,
    /// Whether the target registers this domain.
    pub registered: bool,
}

/// Counts proving complete restore inspection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreInspectionReceiptProjection {
    /// Included manifest domains inspected.
    pub manifest_domains: usize,
    /// Manifest exclusions inspected.
    pub exclusions: usize,
    /// Ordinary domains eligible for selection.
    pub restorable: usize,
    /// Eligible domains requiring migration.
    pub migrations: usize,
    /// Domains eligible for explicit adapter restore.
    pub adapter_restorable: usize,
    /// Included domains blocked from selection.
    pub blocked: usize,
}

/// Side-effect-free inspection of one verified archive.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreInspectionProjection {
    /// Digest over exact selected archive bytes.
    pub archive_sha256: String,
    /// Manifest archive identity.
    pub archive_id: String,
    /// Strict manifest creation time.
    pub created_at: String,
    /// Archive purpose.
    pub kind: String,
    /// Producing application version.
    pub application_version: String,
    /// Producing Longhorn version.
    pub producer_version: String,
    /// Byte-integrity result.
    pub integrity: RestoreIntegrityProjection,
    /// Independent authenticity result.
    pub authenticity: RestoreAuthenticityProjection,
    /// Application and producer compatibility.
    pub identity: RestoreIdentityProjection,
    /// Independent archive consistency groups.
    pub consistency_groups: Vec<RestoreConsistencyGroupProjection>,
    /// Included archive domains.
    pub domains: Vec<RestoreDomainInspectionProjection>,
    /// Explicit archive exclusions.
    pub exclusions: Vec<RestoreExclusionProjection>,
    /// Complete inspection counts.
    pub receipt: RestoreInspectionReceiptProjection,
}

/// Loads and inspects an archive without mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RestoreInspectCommand {
    /// Exact protocol version.
    pub protocol_version: ConfigProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: ConfigRequestId,
    /// Inventory digest or host picker request.
    pub selection: RestoreArchiveSelection,
}

/// Result of archive selection, unlock, and inspection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum RestoreInspectOutcome {
    /// Selection, unlock, and inspection completed without mutation.
    Ready {
        /// Fresh host generation.
        generation: ConfigGeneration,
        /// Complete verified inspection.
        inspection: Box<RestoreInspectionProjection>,
    },
    /// Encryption identity is unavailable.
    Locked {
        /// Redacted host detail.
        detail: String,
    },
    /// Inspection could not produce a safe report.
    Rejected {
        /// Typed refusal.
        rejection: ConfigOperationRejection,
    },
}
