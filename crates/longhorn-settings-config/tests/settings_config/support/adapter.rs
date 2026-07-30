use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use longhorn_core::{SettingsActivationTargetId, SettingsEntryId, SettingsPolicySourceId};
use longhorn_settings::{
    SettingsActivationRequirement, SettingsActivationState, SettingsEditability,
    SettingsEffectiveSource, SettingsMutationOutcome, SettingsMutationTiming, SettingsOpaqueValue,
    SettingsPolicyEffect, SettingsPolicyProjection, SettingsRejection, SettingsRejectionCode,
    SettingsValueProjection,
};
use longhorn_settings_config::{
    SettingsCommittedMutation, SettingsConfigAdapter, SettingsConfigProjection,
    SettingsConfigProjectionError,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Preferences, entry_id};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PolicyMode {
    Editable,
    ForcedTheme,
    ReadOnlyTheme,
}

pub(crate) struct PreferencesAdapter {
    pub(crate) policy: PolicyMode,
    pub(crate) activation_calls: Arc<AtomicUsize>,
}

impl PreferencesAdapter {
    pub(crate) fn new(policy: PolicyMode) -> (Self, Arc<AtomicUsize>) {
        let activation_calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                policy,
                activation_calls: Arc::clone(&activation_calls),
            },
            activation_calls,
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PreferenceIntent {
    entry_id: SettingsEntryId,
    value: Value,
}

impl SettingsConfigAdapter<Preferences> for PreferencesAdapter {
    type Intent = PreferenceIntent;

    fn project(
        &self,
        value: &Preferences,
    ) -> Result<SettingsConfigProjection, SettingsConfigProjectionError> {
        SettingsConfigProjection::new(
            vec![
                theme_projection(value, self.policy),
                volume_projection(value),
                fixed_projection(
                    "preferences:locked",
                    value.locked.as_deref(),
                    SettingsEditability::ReadOnly,
                ),
                fixed_projection(
                    "preferences:hidden",
                    value.hidden.as_deref(),
                    SettingsEditability::Hidden,
                ),
                fixed_projection(
                    "preferences:unsupported",
                    value.unsupported.as_deref(),
                    SettingsEditability::Unsupported,
                ),
            ],
            Vec::new(),
        )
    }

    fn decode_intent(
        &self,
        intent: &SettingsOpaqueValue,
    ) -> Result<Self::Intent, SettingsRejection> {
        if intent.codec_version() != 1 {
            return Err(rejection(SettingsRejectionCode::InvalidIntent));
        }
        serde_json::from_value(intent.value().clone())
            .map_err(|_| rejection(SettingsRejectionCode::InvalidIntent))
    }

    fn targeted_entries(&self, intent: &Self::Intent) -> Vec<SettingsEntryId> {
        vec![intent.entry_id.clone()]
    }

    fn validate_intent(
        &self,
        _current: &Preferences,
        intent: &Self::Intent,
        _projection: &SettingsConfigProjection,
    ) -> Result<(), SettingsRejection> {
        if intent.entry_id == entry_id("preferences:volume") {
            let Some(volume) = intent.value.as_u64() else {
                return Err(rejection(SettingsRejectionCode::InvalidIntent));
            };
            if volume > 10 {
                return Err(rejection(SettingsRejectionCode::PolicyBlocked));
            }
        } else if !intent.value.is_string() {
            return Err(rejection(SettingsRejectionCode::InvalidIntent));
        }
        Ok(())
    }

    fn patch(
        &self,
        current: &mut Preferences,
        intent: &Self::Intent,
    ) -> Result<(), SettingsRejection> {
        match intent.entry_id.as_str() {
            "preferences:theme" => current.theme = intent.value.as_str().map(str::to_owned),
            "preferences:volume" => current.volume = intent.value.as_u64().map(|value| value as u8),
            "preferences:locked" => current.locked = intent.value.as_str().map(str::to_owned),
            "preferences:hidden" => current.hidden = intent.value.as_str().map(str::to_owned),
            "preferences:unsupported" => {
                current.unsupported = intent.value.as_str().map(str::to_owned)
            }
            _ => return Err(rejection(SettingsRejectionCode::InvalidIntent)),
        }
        Ok(())
    }

    fn reset(
        &self,
        current: &mut Preferences,
        entry_ids: &[SettingsEntryId],
    ) -> Result<(), SettingsRejection> {
        for entry_id in entry_ids {
            match entry_id.as_str() {
                "preferences:theme" => current.theme = None,
                "preferences:volume" => current.volume = None,
                "preferences:locked" => current.locked = None,
                "preferences:hidden" => current.hidden = None,
                "preferences:unsupported" => current.unsupported = None,
                _ => return Err(rejection(SettingsRejectionCode::InvalidIntent)),
            }
        }
        Ok(())
    }

    fn activation_after_commit(
        &self,
        _mutation: SettingsCommittedMutation<'_, Self::Intent>,
        _timing: SettingsMutationTiming,
        outcome: SettingsMutationOutcome,
        _committed: &SettingsConfigProjection,
    ) -> Vec<SettingsActivationRequirement> {
        self.activation_calls.fetch_add(1, Ordering::SeqCst);
        if outcome == SettingsMutationOutcome::Changed {
            vec![SettingsActivationRequirement {
                target_id: SettingsActivationTargetId::new("preferences:runtime").unwrap(),
                state: SettingsActivationState::Pending,
            }]
        } else {
            Vec::new()
        }
    }
}

fn theme_projection(value: &Preferences, policy: PolicyMode) -> SettingsValueProjection {
    let (effective, effective_source, policy_projection, editability) = match policy {
        PolicyMode::ForcedTheme => (
            opaque(json!("managed-dark")),
            SettingsEffectiveSource::ManagedPolicy,
            Some(SettingsPolicyProjection {
                source_id: SettingsPolicySourceId::new("administrator:theme").unwrap(),
                effect: SettingsPolicyEffect::Override,
                constraints: None,
            }),
            SettingsEditability::ReadOnly,
        ),
        PolicyMode::ReadOnlyTheme => (
            opaque(json!(value.theme.as_deref().unwrap_or("system"))),
            source(value.theme.is_some()),
            None,
            SettingsEditability::ReadOnly,
        ),
        PolicyMode::Editable => (
            opaque(json!(value.theme.as_deref().unwrap_or("system"))),
            source(value.theme.is_some()),
            None,
            SettingsEditability::Editable,
        ),
    };
    SettingsValueProjection {
        entry_id: entry_id("preferences:theme"),
        configured: value.theme.as_deref().map(|value| opaque(json!(value))),
        effective,
        compiled_default: opaque(json!("system")),
        effective_source,
        policy: policy_projection,
        editability,
        source_diagnostics: Vec::new(),
    }
}

fn volume_projection(value: &Preferences) -> SettingsValueProjection {
    SettingsValueProjection {
        entry_id: entry_id("preferences:volume"),
        configured: value.volume.map(|value| opaque(json!(value))),
        effective: opaque(json!(value.volume.unwrap_or(5))),
        compiled_default: opaque(json!(5)),
        effective_source: source(value.volume.is_some()),
        policy: Some(SettingsPolicyProjection {
            source_id: SettingsPolicySourceId::new("administrator:volume").unwrap(),
            effect: SettingsPolicyEffect::Constraint,
            constraints: Some(opaque(json!({"maximum": 10}))),
        }),
        editability: SettingsEditability::Editable,
        source_diagnostics: Vec::new(),
    }
}

fn fixed_projection(
    id: &str,
    configured: Option<&str>,
    editability: SettingsEditability,
) -> SettingsValueProjection {
    SettingsValueProjection {
        entry_id: entry_id(id),
        configured: configured.map(|value| opaque(json!(value))),
        effective: opaque(json!(configured.unwrap_or("default"))),
        compiled_default: opaque(json!("default")),
        effective_source: source(configured.is_some()),
        policy: None,
        editability,
        source_diagnostics: Vec::new(),
    }
}

fn source(configured: bool) -> SettingsEffectiveSource {
    if configured {
        SettingsEffectiveSource::UserConfiguration
    } else {
        SettingsEffectiveSource::CompiledDefault
    }
}

pub(crate) fn opaque(value: Value) -> SettingsOpaqueValue {
    SettingsOpaqueValue::new(1, value, longhorn_settings::SettingsLimits::default()).unwrap()
}

pub(crate) fn rejection(code: SettingsRejectionCode) -> SettingsRejection {
    SettingsRejection {
        code,
        diagnostic: None,
    }
}
