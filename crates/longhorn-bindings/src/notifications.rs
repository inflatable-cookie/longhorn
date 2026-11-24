use std::error::Error;

use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationLedgerRevision, NotificationProducerToken, NotificationReplacementKey,
    NotificationRequestId, NotificationSourceId,
};
use longhorn_notifications::{
    NOTIFICATION_PROTOCOL_VERSION, NotificationActionProjection, NotificationAuthorityProjection,
    NotificationChangedEvent, NotificationChangedKind, NotificationClearTargetProjection,
    NotificationDraftProjection, NotificationLedgerLimitsProjection, NotificationMutationCommand,
    NotificationMutationReceiptProjection, NotificationMutationResult, NotificationPageProjection,
    NotificationProtocolVersion, NotificationReadStateProjection, NotificationRecordProjection,
    NotificationRejection, NotificationRejectionCode, NotificationRemovalProjection,
    NotificationRemovalReasonProjection, NotificationRetentionClassProjection,
    NotificationSeverity, NotificationSeverityProjection, NotificationSnapshot,
    NotificationSnapshotQuery, NotificationSnapshotResponse,
};
use ts_rs::TS;

use crate::{
    generation::{
        Artifact, DomainArtifacts, GENERATE_TASK, LabelMap, build_domain_artifacts, config,
        exported_declaration, label_module, tagged_variants_json,
    },
    serde_shape::enum_names_json,
};

mod fixture;

// Compile the adapter's wire declarations without linking the Tauri runtime.
#[allow(dead_code)]
#[path = "../../longhorn-tauri-notifications/src/error.rs"]
mod host_error;

const ARTIFACTS: DomainArtifacts = DomainArtifacts {
    protocol: "packages/longhorn/src/notifications/generated/protocol.ts",
    fields: "packages/longhorn/src/notifications/generated/fields.ts",
    fields_constant: "NOTIFICATIONS_FIELDS",
    variant_fields: "packages/longhorn/src/notifications/generated/variant-fields.ts",
    variant_fields_constant: "NOTIFICATION_VARIANT_FIELDS",
    fixture: "fixtures/notifications/protocol-v1.json",
};
const GENERATED_LABELS: &str = "packages/longhorn/src/notifications/generated/labels.ts";

fn render_labels() -> String {
    let entries: Vec<(&str, &str)> = NotificationSeverity::ALL
        .iter()
        .map(|severity| (severity.wire_name(), severity.label()))
        .collect();
    let prefixes: Vec<(&str, &str)> = NotificationSeverity::ALL
        .iter()
        .filter_map(|severity| {
            severity
                .title_prefix()
                .map(|prefix| (severity.wire_name(), prefix))
        })
        .collect();

    let mut rendered = label_module(
        GENERATE_TASK,
        &[LabelMap {
            constant: "NOTIFICATION_SEVERITY_LABELS",
            import: "NotificationSeverityProjection",
            key_type: "NotificationSeverityProjection",
            entries: &entries,
        }],
    );

    // Partial by design: only the severities whose tone cannot carry them
    // appear, so a lookup miss means "the tone says enough". See memo 022, D5.
    rendered.push_str("\n/** Title prefixes for severities the tone cannot distinguish. */\n");
    rendered.push_str("export const NOTIFICATION_SEVERITY_TITLE_PREFIXES: Partial<\n");
    rendered.push_str("  Record<NotificationSeverityProjection, string>\n> = {\n");
    for (name, prefix) in prefixes {
        rendered.push_str("  ");
        rendered.push_str(name);
        rendered.push_str(": \"");
        rendered.push_str(prefix);
        rendered.push_str("\",\n");
    }
    rendered.push_str("};\n");
    rendered
}

/// Renders the notification bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
    let clear_target = NotificationClearTargetProjection::decl(config());
    let mutation = NotificationMutationCommand::decl(config());
    let receipt = NotificationMutationReceiptProjection::decl(config());
    let result = NotificationMutationResult::decl(config());
    let [host_error_code, host_error] =
        crate::serde_shape::error_declarations::<_, host_error::NotificationHostErrorCode>(
            host_error::NotificationHostError::authority_state_unavailable(),
            "NotificationHostError",
            "NotificationHostErrorCode",
        )?;

    let declarations = [
        NotificationActionReferenceId::decl(config()),
        NotificationAuthorityId::decl(config()),
        NotificationCauseId::decl(config()),
        NotificationId::decl(config()),
        NotificationLedgerRevision::decl(config()),
        NotificationProducerToken::decl(config()),
        NotificationReplacementKey::decl(config()),
        NotificationRequestId::decl(config()),
        NotificationSourceId::decl(config()),
        NotificationProtocolVersion::decl(config()),
        NotificationAuthorityProjection::decl(config()),
        NotificationSeverityProjection::decl(config()),
        NotificationReadStateProjection::decl(config()),
        NotificationRetentionClassProjection::decl(config()),
        NotificationActionProjection::decl(config()),
        NotificationDraftProjection::decl(config()),
        NotificationRecordProjection::decl(config()),
        NotificationLedgerLimitsProjection::decl(config()),
        NotificationPageProjection::decl(config()),
        NotificationSnapshot::decl(config()),
        NotificationSnapshotQuery::decl(config()),
        NotificationSnapshotResponse::decl(config()),
        clear_target.clone(),
        mutation.clone(),
        NotificationRemovalReasonProjection::decl(config()),
        NotificationRemovalProjection::decl(config()),
        receipt.clone(),
        NotificationRejectionCode::decl(config()),
        NotificationRejection::decl(config()),
        result.clone(),
        NotificationChangedKind::decl(config()),
        NotificationChangedEvent::decl(config()),
        host_error_code,
        host_error,
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
         // Rust serde types are the wire authority. Executable actions are intentionally absent.\n\n\
         export const NOTIFICATION_PROTOCOL_VERSION = {NOTIFICATION_PROTOCOL_VERSION} as const;\n\
         export const NOTIFICATION_SEVERITIES = {} as const;\n\
         export const NOTIFICATION_READ_STATES = {} as const;\n\
         export const NOTIFICATION_RETENTION_CLASSES = {} as const;\n\
         export const NOTIFICATION_CLEAR_TARGET_KINDS = {} as const;\n\
         export const NOTIFICATION_MUTATION_KINDS = {} as const;\n\
         export const NOTIFICATION_REMOVAL_REASONS = {} as const;\n\
         export const NOTIFICATION_RECEIPT_KINDS = {} as const;\n\
         export const NOTIFICATION_REJECTION_CODES = {} as const;\n\
         export const NOTIFICATION_MUTATION_STATUSES = {} as const;\n\
         export const NOTIFICATION_CHANGED_KINDS = {} as const;\n\n\
         {}\n",
        enum_names_json::<NotificationSeverityProjection>()?,
        enum_names_json::<NotificationReadStateProjection>()?,
        enum_names_json::<NotificationRetentionClassProjection>()?,
        tagged_variants_json(&clear_target)?,
        tagged_variants_json(&mutation)?,
        enum_names_json::<NotificationRemovalReasonProjection>()?,
        tagged_variants_json(&receipt)?,
        enum_names_json::<NotificationRejectionCode>()?,
        tagged_variants_json(&result)?,
        enum_names_json::<NotificationChangedKind>()?,
        declarations.join("\n\n")
    );

    let mut artifacts =
        build_domain_artifacts(&ARTIFACTS, contents, &declarations, fixture::render()?)?;
    artifacts.push(Artifact {
        relative_path: GENERATED_LABELS,
        contents: render_labels(),
    });
    artifacts.push(crate::serde_shape::error_fixture(
        "fixtures/notifications/host-errors.json",
        &[
            host_error::NotificationHostError::authority("Host temporarily unavailable", true),
            host_error::NotificationHostError::authority_state_unavailable(),
        ],
    )?);
    Ok(artifacts)
}
