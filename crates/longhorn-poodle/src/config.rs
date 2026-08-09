//! Projects config restore inspection into Poodle specs.
//!
//! The Svelte tier's equivalent lives in `config/poodle/restore-model.ts` and
//! three `.svelte` pages. Only the model file has a Rust counterpart: the
//! pages hold session state, a transport client, and `crypto.randomUUID()`,
//! none of which are projections. That separation is the point of this module
//! — what is pure fact-to-presentation belongs here once, in the language the
//! facts are already written in.

use longhorn_config::{
    RestoreAuthenticityProjection, RestoreDomainCompatibilityProjection,
    RestoreDomainInspectionProjection, RestoreIdentityStatusProjection,
    RestoreInspectionProjection, RestoreIntegrityProjection,
};
use poodle_specs::{ChoiceOption, DetailItemSpec, RadioGroupSpec};

/// Restore choice values, shared with the Svelte tier's `RestoreChoice`.
pub const RESTORE_CHOICE_USE_ARCHIVE: &str = "useArchive";
/// Keep the target's current data for this domain.
pub const RESTORE_CHOICE_KEEP_CURRENT: &str = "keepCurrent";

/// Whether the archive's copy of this domain can be chosen at all.
///
/// Ready and migration-required are the two selectable classifications;
/// everything else is either blocked or a separate adapter-owned operation
/// with its own receipt, which is not this radio group's business.
#[must_use]
pub fn can_use_archive(domain: &RestoreDomainInspectionProjection) -> bool {
    matches!(
        domain.compatibility,
        RestoreDomainCompatibilityProjection::Ready
            | RestoreDomainCompatibilityProjection::MigrationRequired { .. }
    )
}

/// Renders one compatibility classification as operator-facing text.
///
/// Delegates to `longhorn-config`. The wording is a property of the
/// classification, not of this projection, and it is the source the generated
/// TypeScript map is built from — so both backends say the same thing by
/// construction rather than by review. See memo 022, D2, and Card 170.
#[must_use]
pub fn compatibility_label(compatibility: &RestoreDomainCompatibilityProjection) -> String {
    compatibility.label()
}

/// Renders byte-integrity state.
///
/// Named rather than printed: the wire form is a serde encoding and not a
/// sentence. The wording lives in `longhorn-config`.
#[must_use]
pub const fn integrity_label(integrity: RestoreIntegrityProjection) -> &'static str {
    integrity.label()
}

/// Renders authenticity state, which is deliberately not integrity.
#[must_use]
pub const fn authenticity_label(authenticity: RestoreAuthenticityProjection) -> &'static str {
    authenticity.label()
}

/// Renders one identity comparison, naming both sides when they differ.
#[must_use]
pub fn identity_label(status: &RestoreIdentityStatusProjection) -> String {
    match status {
        RestoreIdentityStatusProjection::Compatible => "Compatible".to_owned(),
        RestoreIdentityStatusProjection::Mismatch { expected, archive } => {
            format!("Mismatch: host expects {expected}, archive declares {archive}")
        }
    }
}

/// The per-domain restore choice, with unselectable options disabled.
///
/// The empty value means undecided, and it is absent from the options: a
/// domain starts with no choice and the group renders nothing selected. An
/// explicit "undecided" option would let an operator move backwards into a
/// state the plan cannot act on.
#[must_use]
pub fn restore_choice_group(
    domain: &RestoreDomainInspectionProjection,
    chosen: Option<&str>,
) -> RadioGroupSpec {
    let archive = ChoiceOption::new(RESTORE_CHOICE_USE_ARCHIVE, "Use the archive's copy")
        .with_disabled(!can_use_archive(domain));
    let current = ChoiceOption::new(RESTORE_CHOICE_KEEP_CURRENT, "Keep the current data");

    // `aria_label` is the one `RadioGroupSpec` field with no `with_` builder,
    // so it is set directly. The field is public and the asymmetry is
    // Poodle's, not a gap in the spec.
    let mut spec = RadioGroupSpec::new(vec![archive, current]).with_name(domain.domain_id.clone());
    spec.aria_label = Some(format!("Restore choice for {}", domain.domain_id));
    if let Some(value) = chosen {
        spec = spec.with_value(value);
    }
    spec
}

/// The verified-archive detail block.
///
/// Ordered as the Svelte page orders it, because the order is a claim: bytes
/// first, then who vouched for them, then whose application they came from.
/// The digest is last and truncated, since it is evidence to copy rather than
/// to read.
#[must_use]
pub fn archive_details(inspection: &RestoreInspectionProjection) -> Vec<DetailItemSpec> {
    vec![
        DetailItemSpec::new("Archive").with_value(inspection.archive_id.clone()),
        DetailItemSpec::new("Created").with_value(inspection.created_at.clone()),
        DetailItemSpec::new("Integrity").with_value(integrity_label(inspection.integrity)),
        DetailItemSpec::new("Authenticity").with_value(authenticity_label(inspection.authenticity)),
        DetailItemSpec::new("Application identity")
            .with_value(identity_label(&inspection.identity.application)),
        DetailItemSpec::new("Producer identity")
            .with_value(identity_label(&inspection.identity.producer)),
        DetailItemSpec::new("Archive digest")
            .with_value(inspection.archive_sha256.clone())
            .with_truncate_value(true),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(
        compatibility: RestoreDomainCompatibilityProjection,
    ) -> RestoreDomainInspectionProjection {
        RestoreDomainInspectionProjection {
            domain_id: "settings".to_owned(),
            storage_class: "sqlite".to_owned(),
            consistency_group: "primary".to_owned(),
            adapter: "builtin".to_owned(),
            source_state: "present".to_owned(),
            source_schema_version: Some(3),
            target_schema_version: Some(4),
            compatibility,
        }
    }

    #[test]
    fn only_ready_and_migration_required_are_selectable() {
        assert!(can_use_archive(&domain(
            RestoreDomainCompatibilityProjection::Ready
        )));
        assert!(can_use_archive(&domain(
            RestoreDomainCompatibilityProjection::MigrationRequired { from: 3, to: 4 }
        )));
        assert!(!can_use_archive(&domain(
            RestoreDomainCompatibilityProjection::UnknownDomain
        )));
        // Adapter-ready is a separate receipted operation, not a radio choice.
        assert!(!can_use_archive(&domain(
            RestoreDomainCompatibilityProjection::CustomAdapterUnavailable {
                adapter: "vault".to_owned()
            }
        )));
    }

    #[test]
    fn an_unselectable_domain_disables_only_the_archive_option() {
        let spec = restore_choice_group(
            &domain(RestoreDomainCompatibilityProjection::DescriptorMismatch),
            None,
        );

        assert_eq!(spec.options.len(), 2);
        assert!(spec.options[0].is_disabled);
        assert!(!spec.options[1].is_disabled);
        // No choice yet means nothing selected, not a third "undecided" option.
        assert!(spec.value.is_none());
    }

    #[test]
    fn labels_name_both_sides_of_a_migration() {
        let label = compatibility_label(&RestoreDomainCompatibilityProjection::MigrationRequired {
            from: 3,
            to: 7,
        });
        assert!(label.contains('3'), "{label}");
        assert!(label.contains('7'), "{label}");
    }

    #[test]
    fn an_identity_mismatch_names_the_host_and_the_archive() {
        let label = identity_label(&RestoreIdentityStatusProjection::Mismatch {
            expected: "com.example.host".to_owned(),
            archive: "com.example.other".to_owned(),
        });
        assert!(label.contains("com.example.host"), "{label}");
        assert!(label.contains("com.example.other"), "{label}");
    }
}
