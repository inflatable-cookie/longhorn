use longhorn_core::{
    SettingsActivationTargetId, SettingsApplyUnitId, SettingsAuthorityToken, SettingsEntryId,
    SettingsPageId, SettingsPolicySourceId, SettingsRequestId, SettingsScopeId,
};
use longhorn_settings::{
    SettingsActivationRequirement, SettingsActivationState, SettingsApplyCommand,
    SettingsAuthorityExpectation, SettingsConflict, SettingsDurabilityEvidence,
    SettingsEditability, SettingsEffectiveSource, SettingsLimits, SettingsLoadCommand,
    SettingsMutationOutcome, SettingsMutationReceipt, SettingsMutationResult, SettingsOpaqueValue,
    SettingsOpaqueValueError, SettingsPolicyEffect, SettingsPolicyProjection,
    SettingsProtocolError, SettingsProtocolVersion, SettingsRegistryGeneration,
    SettingsResetCommand, SettingsScopeRevision, SettingsScopeSnapshot, SettingsValueProjection,
};
use serde_json::json;

#[test]
fn opaque_values_are_versioned_bounded_and_strict() {
    let value =
        SettingsOpaqueValue::new(3, json!({"theme": "dark"}), SettingsLimits::default()).unwrap();
    let encoded = serde_json::to_string(&value).unwrap();
    assert_eq!(
        serde_json::from_str::<SettingsOpaqueValue>(&encoded).unwrap(),
        value
    );
    assert_eq!(value.codec_version(), 3);
    assert!(value.encoded_bytes() > 0);

    assert_eq!(
        SettingsOpaqueValue::new(0, json!(null), SettingsLimits::default()),
        Err(SettingsOpaqueValueError::InvalidCodecVersion)
    );

    let limits = SettingsLimits {
        maximum_opaque_value_bytes: 64,
        ..SettingsLimits::default()
    };
    assert!(matches!(
        SettingsOpaqueValue::new(1, json!({"payload": "x".repeat(128)}), limits),
        Err(SettingsOpaqueValueError::TooLarge { .. })
    ));
    assert!(
        serde_json::from_str::<SettingsOpaqueValue>(
            r#"{"codecVersion":1,"value":null,"unexpected":true}"#
        )
        .is_err()
    );
}

#[test]
fn projection_keeps_configured_effective_default_and_policy_distinct() {
    let configured = opaque(json!("user"));
    let effective = opaque(json!("managed"));
    let compiled_default = opaque(json!("default"));
    let projection = SettingsValueProjection {
        entry_id: SettingsEntryId::new("appearance:theme").unwrap(),
        configured: Some(configured.clone()),
        effective: effective.clone(),
        compiled_default: compiled_default.clone(),
        effective_source: SettingsEffectiveSource::ManagedPolicy,
        policy: Some(SettingsPolicyProjection {
            source_id: SettingsPolicySourceId::new("policy:administrator").unwrap(),
            effect: SettingsPolicyEffect::Override,
            constraints: None,
        }),
        editability: SettingsEditability::ReadOnly,
        source_diagnostics: vec![],
    };

    let decoded: SettingsValueProjection =
        serde_json::from_str(&serde_json::to_string(&projection).unwrap()).unwrap();
    assert_eq!(decoded.configured, Some(configured));
    assert_eq!(decoded.effective, effective);
    assert_eq!(decoded.compiled_default, compiled_default);
    assert!(decoded.policy.is_some());
}

#[test]
fn apply_and_reset_commands_bind_generation_revision_and_token() {
    let authority = authority(11, 23, "token:before");
    let apply = SettingsApplyCommand {
        protocol_version: SettingsProtocolVersion::CURRENT,
        request_id: SettingsRequestId::new("request:apply").unwrap(),
        page_id: SettingsPageId::new("app:page").unwrap(),
        apply_unit_id: SettingsApplyUnitId::new("app:apply").unwrap(),
        scope_id: SettingsScopeId::new("app:scope").unwrap(),
        authority: authority.clone(),
        intent: opaque(json!({"set": {"theme": "dark"}})),
    };
    let reset = SettingsResetCommand {
        protocol_version: SettingsProtocolVersion::CURRENT,
        request_id: SettingsRequestId::new("request:reset").unwrap(),
        page_id: apply.page_id.clone(),
        apply_unit_id: apply.apply_unit_id.clone(),
        scope_id: apply.scope_id.clone(),
        authority: authority.clone(),
        entry_ids: vec![SettingsEntryId::new("appearance:theme").unwrap()],
    };

    let decoded_apply: SettingsApplyCommand =
        serde_json::from_str(&serde_json::to_string(&apply).unwrap()).unwrap();
    let decoded_reset: SettingsResetCommand =
        serde_json::from_str(&serde_json::to_string(&reset).unwrap()).unwrap();
    assert_eq!(decoded_apply.authority, authority);
    assert_eq!(decoded_reset.authority.registry_generation.get(), 11);
    assert_eq!(decoded_reset.authority.scope_revision.get(), 23);
    assert_eq!(
        decoded_reset.authority.authority_token.as_str(),
        "token:before"
    );
}

#[test]
fn load_protocol_rejects_future_versions_and_unknown_fields() {
    let load = SettingsLoadCommand {
        protocol_version: SettingsProtocolVersion::CURRENT,
        request_id: SettingsRequestId::new("request:load").unwrap(),
        registry_generation: SettingsRegistryGeneration::new(3),
        scope_id: SettingsScopeId::new("app:scope").unwrap(),
        known_authority: Some(authority(3, 8, "token:known")),
    };
    let encoded = serde_json::to_value(load).unwrap();

    let mut future = encoded.clone();
    future["protocolVersion"] = json!(2);
    assert!(serde_json::from_value::<SettingsLoadCommand>(future).is_err());

    let mut unknown = encoded;
    unknown["consumerGuess"] = json!(true);
    assert!(serde_json::from_value::<SettingsLoadCommand>(unknown).is_err());
}

#[test]
fn receipt_keeps_durability_and_activation_separate() {
    let previous = authority(5, 41, "token:before");
    let committed = authority(5, 42, "token:after");
    let activation = SettingsActivationRequirement {
        target_id: SettingsActivationTargetId::new("app:runtime").unwrap(),
        state: SettingsActivationState::Pending,
    };
    let snapshot = SettingsScopeSnapshot {
        protocol_version: SettingsProtocolVersion::CURRENT,
        scope_id: SettingsScopeId::new("app:scope").unwrap(),
        authority: committed.clone(),
        values: vec![],
        recovery: None,
        activation_requirements: vec![activation.clone()],
    };
    let result = SettingsMutationResult::Applied {
        snapshot,
        receipt: SettingsMutationReceipt {
            request_id: SettingsRequestId::new("request:apply").unwrap(),
            page_id: SettingsPageId::new("app:page").unwrap(),
            apply_unit_id: SettingsApplyUnitId::new("app:apply").unwrap(),
            scope_id: SettingsScopeId::new("app:scope").unwrap(),
            previous_authority: previous,
            committed_authority: committed,
            outcome: SettingsMutationOutcome::Changed,
            durability: SettingsDurabilityEvidence::Confirmed { evidence: None },
            activation_requirements: vec![activation],
        },
    };

    let decoded: SettingsMutationResult =
        serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    assert_eq!(decoded, result);
}

#[test]
fn conflict_returns_fresh_authority_without_overwrite() {
    let conflict = SettingsConflict {
        expected: authority(2, 8, "token:stale"),
        actual: authority(2, 9, "token:current"),
    };
    assert_ne!(conflict.expected, conflict.actual);
    assert_eq!(
        conflict.expected.registry_generation,
        conflict.actual.registry_generation
    );
}

#[test]
fn scope_revision_never_wraps() {
    assert_eq!(
        SettingsScopeRevision::INITIAL.checked_next().unwrap().get(),
        1
    );
    assert_eq!(
        SettingsScopeRevision::new(u64::MAX).checked_next(),
        Err(SettingsProtocolError::ScopeRevisionOverflow)
    );
}

fn authority(generation: u64, revision: u64, token: &str) -> SettingsAuthorityExpectation {
    SettingsAuthorityExpectation {
        registry_generation: SettingsRegistryGeneration::new(generation),
        scope_revision: SettingsScopeRevision::new(revision),
        authority_token: SettingsAuthorityToken::new(token).unwrap(),
    }
}

fn opaque(value: serde_json::Value) -> SettingsOpaqueValue {
    SettingsOpaqueValue::new(1, value, SettingsLimits::default()).unwrap()
}
