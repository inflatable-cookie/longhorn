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

impl RestoreIntegrityProjection {
    /// Every integrity state.
    pub const ALL: [Self; 1] = [Self::Verified];

    /// The wire name, which is also the generated map's key.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Verified => "verified",
        }
    }

    /// The operator-facing name.
    ///
    /// Exists because a surface that renders the wire form shows an operator
    /// `verified` in a restore dialog — an implementation detail at the
    /// moment they most need plain language. See memo 022, D1.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Verified => "Verified",
        }
    }
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

impl RestoreAuthenticityProjection {
    /// Every authenticity state.
    pub const ALL: [Self; 2] = [Self::Unauthenticated, Self::Authenticated];

    /// The wire name, which is also the generated map's key.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::Authenticated => "authenticated",
        }
    }

    /// The operator-facing name.
    ///
    /// Deliberately not "Unverified": authenticity is not integrity, and a
    /// plaintext archive whose checksums all pass is authentic-unclaimed
    /// rather than suspect.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unauthenticated => "Unauthenticated",
            Self::Authenticated => "Authenticated",
        }
    }
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

/// Fills `{name}` placeholders in a label template.
///
/// One substitution rule and deliberately not a template language: a
/// placeholder is a name in braces, and anything else is literal. Both this
/// and the generated TypeScript interpolate the *same* templates, so the two
/// backends cannot word a label differently — which is the whole point of
/// carrying templates rather than finished strings.
///
/// An unknown placeholder is left as written rather than blanked, so a
/// mistake shows up as `{typo}` on screen instead of a hole.
#[must_use]
pub fn render_label_template(template: &str, fields: &[(&str, &str)]) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        rendered.push_str(&rest[..open]);
        rest = &rest[open..];

        let Some(close) = rest.find('}') else {
            break;
        };
        let name = &rest[1..close];
        match fields.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => rendered.push_str(value),
            None => rendered.push_str(&rest[..=close]),
        }
        rest = &rest[close + 1..];
    }

    rendered.push_str(rest);
    rendered
}

impl RestoreDomainCompatibilityProjection {
    /// Every classification's wire name and label template, in declaration
    /// order.
    ///
    /// The generated TypeScript map is built from this, so a fourteenth
    /// classification that is not added here fails the bindings gate. That is
    /// the drift memo 022 recorded as D2: previously only the Rust map failed
    /// to compile, and the TypeScript one silently returned `undefined`.
    pub const TEMPLATES: [(&'static str, &'static str); 13] = [
        ("ready", "Ready"),
        (
            "migrationRequired",
            "Migration required ({from} \u{2192} {to})",
        ),
        ("unknownDomain", "Unknown domain"),
        ("descriptorMismatch", "Descriptor mismatch"),
        ("domainCodeUnavailable", "Domain code unavailable"),
        ("policyExcluded", "Policy excluded: {reason}"),
        ("customAdapterUnavailable", "Adapter unavailable: {adapter}"),
        ("customAdapterReady", "Custom adapter ready: {adapter}"),
        (
            "customAdapterRejected",
            "Adapter rejected: {adapter} \u{2014} {detail}",
        ),
        ("targetUnavailable", "Target unavailable: {reason}"),
        ("sourcePreserved", "Source preserved: {issue}"),
        ("sourceRejected", "Source rejected: {issue}"),
        (
            "targetPreparationFailed",
            "Target preparation failed: {detail}",
        ),
    ];

    /// This classification's wire name.
    #[must_use]
    pub const fn wire_name(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::MigrationRequired { .. } => "migrationRequired",
            Self::UnknownDomain => "unknownDomain",
            Self::DescriptorMismatch => "descriptorMismatch",
            Self::DomainCodeUnavailable => "domainCodeUnavailable",
            Self::PolicyExcluded { .. } => "policyExcluded",
            Self::CustomAdapterUnavailable { .. } => "customAdapterUnavailable",
            Self::CustomAdapterReady { .. } => "customAdapterReady",
            Self::CustomAdapterRejected { .. } => "customAdapterRejected",
            Self::TargetUnavailable { .. } => "targetUnavailable",
            Self::SourcePreserved { .. } => "sourcePreserved",
            Self::SourceRejected { .. } => "sourceRejected",
            Self::TargetPreparationFailed { .. } => "targetPreparationFailed",
        }
    }

    /// This classification's label template, before its fields are filled.
    #[must_use]
    pub fn label_template(&self) -> &'static str {
        let wanted = self.wire_name();
        Self::TEMPLATES
            .iter()
            .find(|(name, _)| *name == wanted)
            .map_or("", |(_, template)| *template)
    }

    /// The operator-facing label, rendered from this classification's own
    /// template and fields.
    ///
    /// Rendered rather than written out arm by arm, so the string a Rust
    /// surface shows and the string a TypeScript surface shows come from one
    /// source. The stable values a classification carries — `reason`,
    /// `adapter`, `issue`, `detail` — are non-secret by construction and safe
    /// to show.
    #[must_use]
    pub fn label(&self) -> String {
        let template = self.label_template();
        match self {
            Self::Ready
            | Self::UnknownDomain
            | Self::DescriptorMismatch
            | Self::DomainCodeUnavailable => template.to_owned(),
            Self::MigrationRequired { from, to } => render_label_template(
                template,
                &[("from", &from.to_string()), ("to", &to.to_string())],
            ),
            Self::PolicyExcluded { reason } | Self::TargetUnavailable { reason } => {
                render_label_template(template, &[("reason", reason)])
            }
            Self::CustomAdapterUnavailable { adapter }
            | Self::CustomAdapterReady { adapter, .. } => {
                render_label_template(template, &[("adapter", adapter)])
            }
            Self::CustomAdapterRejected { adapter, detail } => {
                render_label_template(template, &[("adapter", adapter), ("detail", detail)])
            }
            Self::SourcePreserved { issue } | Self::SourceRejected { issue } => {
                render_label_template(template, &[("issue", issue)])
            }
            Self::TargetPreparationFailed { detail } => {
                render_label_template(template, &[("detail", detail)])
            }
        }
    }
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

#[cfg(test)]
mod label_tests {
    use super::*;

    fn every_classification() -> Vec<RestoreDomainCompatibilityProjection> {
        use RestoreDomainCompatibilityProjection as C;
        vec![
            C::Ready,
            C::MigrationRequired { from: 3, to: 7 },
            C::UnknownDomain,
            C::DescriptorMismatch,
            C::DomainCodeUnavailable,
            C::PolicyExcluded {
                reason: "beta".to_owned(),
            },
            C::CustomAdapterUnavailable {
                adapter: "vault".to_owned(),
            },
            C::CustomAdapterReady {
                adapter: "vault".to_owned(),
                participation: RestoreAdapterParticipationProjection::Separate,
                confirmation_digest: "abc".to_owned(),
            },
            C::CustomAdapterRejected {
                adapter: "vault".to_owned(),
                detail: "checksum".to_owned(),
            },
            C::TargetUnavailable {
                reason: "sealed".to_owned(),
            },
            C::SourcePreserved {
                issue: "truncated".to_owned(),
            },
            C::SourceRejected {
                issue: "corrupt".to_owned(),
            },
            C::TargetPreparationFailed {
                detail: "no space".to_owned(),
            },
        ]
    }

    #[test]
    fn every_classification_has_a_template_and_leaves_no_placeholder_unfilled() {
        // The failure this guards: a template naming a field the arm does not
        // supply renders `{typo}` to an operator.
        let classifications = every_classification();
        assert_eq!(
            classifications.len(),
            RestoreDomainCompatibilityProjection::TEMPLATES.len()
        );

        for classification in &classifications {
            let label = classification.label();
            assert!(!label.is_empty(), "{classification:?}");
            assert!(!label.contains('{'), "{label}");
            assert!(!label.contains('}'), "{label}");
        }
    }

    #[test]
    fn the_template_table_covers_exactly_the_wire_names() {
        // The table is what the generated TypeScript is built from, so a
        // classification missing from it would ship a blank label.
        for classification in every_classification() {
            let wanted = classification.wire_name();
            assert!(
                RestoreDomainCompatibilityProjection::TEMPLATES
                    .iter()
                    .any(|(name, _)| *name == wanted),
                "{wanted}"
            );
        }
    }

    #[test]
    fn a_migration_label_names_both_schemas() {
        let label =
            RestoreDomainCompatibilityProjection::MigrationRequired { from: 3, to: 7 }.label();
        assert_eq!(label, "Migration required (3 \u{2192} 7)");
    }

    #[test]
    fn an_unknown_placeholder_stays_visible_rather_than_blank() {
        // A mistake should show up as `{typo}` on screen, not as a hole that
        // reads like intentional wording.
        assert_eq!(
            render_label_template("a {known} b {missing} c", &[("known", "X")]),
            "a X b {missing} c"
        );
    }

    #[test]
    fn a_template_with_no_placeholders_is_returned_intact() {
        assert_eq!(render_label_template("Ready", &[]), "Ready");
    }
}
