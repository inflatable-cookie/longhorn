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

use crate::generation::{
    Artifact, GenerationMode, LabelMap, apply, config, exported_declaration, field_map,
    label_module, string_union_variants, tagged_variants, variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/notifications/generated/protocol.ts";
const GOLDEN_FIXTURE: &str = "fixtures/notifications/protocol-v1.json";
const GENERATED_FIELDS: &str = "packages/longhorn/src/notifications/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str =
    "packages/longhorn/src/notifications/generated/variant-fields.ts";

struct RenderedProtocol {
    contents: String,
    fields: String,
    variant_fields: String,
}
const GENERATED_LABELS: &str = "packages/longhorn/src/notifications/generated/labels.ts";

/// Generates or checks the notification bindings and golden fixtures.
pub fn run(mode: GenerationMode) -> Result<(), Box<dyn Error>> {
    let protocol = render_protocol()?;
    apply(
        "notifications",
        "generate:notifications",
        mode,
        &[
            Artifact {
                relative_path: GENERATED_PROTOCOL,
                contents: protocol.contents,
            },
            Artifact {
                relative_path: GENERATED_FIELDS,
                contents: protocol.fields,
            },
            Artifact {
                relative_path: GENERATED_VARIANT_FIELDS,
                contents: protocol.variant_fields,
            },
            Artifact {
                relative_path: GOLDEN_FIXTURE,
                contents: fixture::render()?,
            },
            Artifact {
                relative_path: GENERATED_LABELS,
                contents: render_labels(),
            },
        ],
    )
}

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
        "generate:notifications",
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

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
    let severity = NotificationSeverityProjection::decl(config());
    let read_state = NotificationReadStateProjection::decl(config());
    let retention = NotificationRetentionClassProjection::decl(config());
    let clear_target = NotificationClearTargetProjection::decl(config());
    let mutation = NotificationMutationCommand::decl(config());
    let removal_reason = NotificationRemovalReasonProjection::decl(config());
    let receipt = NotificationMutationReceiptProjection::decl(config());
    let rejection_code = NotificationRejectionCode::decl(config());
    let result = NotificationMutationResult::decl(config());
    let changed_kind = NotificationChangedKind::decl(config());
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
        severity.clone(),
        read_state.clone(),
        retention.clone(),
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
        removal_reason.clone(),
        NotificationRemovalProjection::decl(config()),
        receipt.clone(),
        rejection_code.clone(),
        NotificationRejection::decl(config()),
        result.clone(),
        changed_kind.clone(),
        NotificationChangedEvent::decl(config()),
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy generate:notifications`; do not edit.\n\
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
        serde_json::to_string(&string_union_variants(&severity)?)?,
        serde_json::to_string(&string_union_variants(&read_state)?)?,
        serde_json::to_string(&string_union_variants(&retention)?)?,
        serde_json::to_string(&tagged_variants(&clear_target, "kind")?)?,
        serde_json::to_string(&tagged_variants(&mutation, "kind")?)?,
        serde_json::to_string(&string_union_variants(&removal_reason)?)?,
        serde_json::to_string(&tagged_variants(&receipt, "kind")?)?,
        serde_json::to_string(&string_union_variants(&rejection_code)?)?,
        serde_json::to_string(&tagged_variants(&result, "status")?)?,
        serde_json::to_string(&string_union_variants(&changed_kind)?)?,
        declarations.join("\n\n")
    );
    let (fields, _skipped) = field_map(
        "generate:notifications",
        "NOTIFICATIONS_FIELDS",
        &declarations,
    );

    let variant_fields = variant_field_map(
        "generate:notifications",
        "NOTIFICATION_VARIANT_FIELDS",
        &declarations,
    );

    Ok(RenderedProtocol {
        contents,

        fields,

        variant_fields,
    })
}
