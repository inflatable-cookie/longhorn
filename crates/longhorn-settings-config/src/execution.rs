use std::collections::BTreeSet;

use longhorn_config::{ConfigDomain, Durability};
use longhorn_core::SettingsEntryId;
use longhorn_settings::{
    SettingsDurabilityEvidence, SettingsEditability, SettingsMutationResult, SettingsOpaqueValue,
    SettingsPolicyEffect, SettingsRejection, SettingsRejectionCode, SettingsScopeSnapshot,
};
use serde_json::json;

use crate::{
    ConfigSettingsApplyUnit, SettingsConfigAdapter, SettingsConfigError, validation::rejection,
};

pub(super) enum CheckAbort {
    Result(Box<SettingsMutationResult>),
    Operational(SettingsConfigError),
}

impl From<SettingsConfigError> for CheckAbort {
    fn from(error: SettingsConfigError) -> Self {
        Self::Operational(error)
    }
}

pub(super) fn rejected(
    rejection: SettingsRejection,
    snapshot: &SettingsScopeSnapshot,
) -> CheckAbort {
    CheckAbort::Result(Box::new(SettingsMutationResult::Rejected {
        rejection,
        snapshot: Some(snapshot.clone()),
    }))
}

pub(super) fn guard_targets(
    projected: &[longhorn_settings::SettingsValueProjection],
    targets: &[SettingsEntryId],
) -> Result<(), SettingsRejection> {
    if targets.is_empty() {
        return Err(rejection(SettingsRejectionCode::InvalidIntent));
    }
    let mut unique = BTreeSet::new();
    for target in targets {
        if !unique.insert(target) {
            return Err(rejection(SettingsRejectionCode::InvalidIntent));
        }
        let Some(value) = projected.iter().find(|value| &value.entry_id == target) else {
            return Err(rejection(SettingsRejectionCode::InvalidIntent));
        };
        if matches!(
            value.policy.as_ref().map(|policy| policy.effect),
            Some(SettingsPolicyEffect::Override)
        ) {
            return Err(rejection(SettingsRejectionCode::PolicyBlocked));
        }
        let code = match value.editability {
            SettingsEditability::Editable => None,
            SettingsEditability::ReadOnly => Some(SettingsRejectionCode::ReadOnly),
            SettingsEditability::Hidden => Some(SettingsRejectionCode::Hidden),
            SettingsEditability::Unsupported => Some(SettingsRejectionCode::Unsupported),
        };
        if let Some(code) = code {
            return Err(rejection(code));
        }
    }
    Ok(())
}

pub(super) struct DurabilityMaterial {
    file_synced: SettingsOpaqueValue,
    directory_synced: SettingsOpaqueValue,
}

impl DurabilityMaterial {
    pub(super) fn for_kind(&self, durability: Durability) -> SettingsDurabilityEvidence {
        let evidence = match durability {
            Durability::FileSynced => self.file_synced.clone(),
            Durability::FileAndDirectorySynced => self.directory_synced.clone(),
        };
        SettingsDurabilityEvidence::Confirmed {
            evidence: Some(evidence),
        }
    }
}

impl<D, A> ConfigSettingsApplyUnit<D, A>
where
    D: ConfigDomain,
    A: SettingsConfigAdapter<D::Value>,
{
    pub(super) fn durability_material(&self) -> Result<DurabilityMaterial, SettingsConfigError> {
        let file_synced =
            SettingsOpaqueValue::new(1, json!({"configDurability": "fileSynced"}), self.limits)
                .map_err(|error| SettingsConfigError::AuthorityEncoding(error.to_string()))?;
        let directory_synced = SettingsOpaqueValue::new(
            1,
            json!({"configDurability": "fileAndDirectorySynced"}),
            self.limits,
        )
        .map_err(|error| SettingsConfigError::AuthorityEncoding(error.to_string()))?;
        Ok(DurabilityMaterial {
            file_synced,
            directory_synced,
        })
    }
}
