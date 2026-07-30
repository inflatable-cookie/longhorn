use longhorn_core::SettingsAuthorityToken;
use longhorn_settings::{
    SettingsActivationRequirement, SettingsActivationState, SettingsAuthorityExpectation,
    SettingsEditability, SettingsEffectiveSource, SettingsLimits, SettingsOpaqueValue,
    SettingsPolicyEffect, SettingsPolicyProjection, SettingsProtocolVersion, SettingsRecoveryCode,
    SettingsRecoveryState, SettingsRegistryGeneration, SettingsScopeRevision,
    SettingsScopeSnapshot, SettingsSourceDiagnostic, SettingsValueProjection,
};
use serde_json::{Value, json};

use super::ids::{activation_id, entry_id, policy_id, scope_id};

pub(super) fn snapshot(
    limits: SettingsLimits,
    authority: SettingsAuthorityExpectation,
) -> SettingsScopeSnapshot {
    SettingsScopeSnapshot {
        protocol_version: SettingsProtocolVersion::CURRENT,
        scope_id: scope_id("app:preferences"),
        authority,
        values: vec![
            value_projection(
                limits,
                "audio:output",
                SettingsEffectiveSource::UserConfiguration,
                SettingsEditability::Editable,
                None,
            ),
            value_projection(
                limits,
                "audio:managed",
                SettingsEffectiveSource::ManagedPolicy,
                SettingsEditability::ReadOnly,
                Some(SettingsPolicyProjection {
                    source_id: policy_id("policy:administrator"),
                    effect: SettingsPolicyEffect::Override,
                    constraints: None,
                }),
            ),
            value_projection(
                limits,
                "audio:hidden",
                SettingsEffectiveSource::CompiledDefault,
                SettingsEditability::Hidden,
                Some(SettingsPolicyProjection {
                    source_id: policy_id("policy:administrator"),
                    effect: SettingsPolicyEffect::Constraint,
                    constraints: Some(opaque(limits, json!({"maximum": 100}))),
                }),
            ),
            value_projection(
                limits,
                "audio:unsupported",
                SettingsEffectiveSource::CompiledDefault,
                SettingsEditability::Unsupported,
                None,
            ),
        ],
        recovery: None,
        activation_requirements: activation_requirements(),
    }
}

fn value_projection(
    limits: SettingsLimits,
    id: &str,
    source: SettingsEffectiveSource,
    editability: SettingsEditability,
    policy: Option<SettingsPolicyProjection>,
) -> SettingsValueProjection {
    SettingsValueProjection {
        entry_id: entry_id(id),
        configured: Some(opaque(limits, json!({"selected": "device:main"}))),
        effective: opaque(limits, json!({"selected": "device:main"})),
        compiled_default: opaque(limits, json!({"selected": "device:system"})),
        effective_source: source,
        policy,
        editability,
        source_diagnostics: vec![SettingsSourceDiagnostic {
            code: "fixture".into(),
            detail: Some(opaque(limits, json!({"source": "test"}))),
        }],
    }
}

pub(super) fn recovery_states(limits: SettingsLimits) -> Vec<SettingsRecoveryState> {
    [
        SettingsRecoveryCode::Corrupt,
        SettingsRecoveryCode::FutureSchema,
        SettingsRecoveryCode::AuthorityUnavailable,
        SettingsRecoveryCode::RecoveryInProgress,
        SettingsRecoveryCode::RecoveryRequired,
    ]
    .into_iter()
    .map(|code| SettingsRecoveryState {
        code,
        diagnostic: Some(opaque(limits, json!({"fixture": true}))),
    })
    .collect()
}

pub(super) fn activation_requirements() -> Vec<SettingsActivationRequirement> {
    vec![
        SettingsActivationRequirement {
            target_id: activation_id("activation:app"),
            state: SettingsActivationState::Pending,
        },
        SettingsActivationRequirement {
            target_id: activation_id("activation:audio"),
            state: SettingsActivationState::Satisfied,
        },
    ]
}

pub(super) fn authority(revision: u64, token: &str) -> SettingsAuthorityExpectation {
    SettingsAuthorityExpectation {
        registry_generation: SettingsRegistryGeneration::new(7),
        scope_revision: SettingsScopeRevision::new(revision),
        authority_token: SettingsAuthorityToken::new(token).unwrap(),
    }
}

pub(super) fn opaque(limits: SettingsLimits, value: Value) -> SettingsOpaqueValue {
    SettingsOpaqueValue::new(1, value, limits).unwrap()
}
