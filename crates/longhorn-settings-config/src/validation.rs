use longhorn_config::ConfigDomain;
use longhorn_core::SettingsRequestId;
use longhorn_settings::{
    SettingsApplyCommand, SettingsMutationResult, SettingsRejection, SettingsRejectionCode,
    SettingsResetCommand,
};

use crate::{ConfigSettingsApplyUnit, SettingsConfigAdapter};

impl<D, A> ConfigSettingsApplyUnit<D, A>
where
    D: ConfigDomain,
    A: SettingsConfigAdapter<D::Value>,
{
    pub(super) fn reject_apply_envelope(
        &self,
        command: &SettingsApplyCommand,
    ) -> Option<SettingsMutationResult> {
        self.reject_envelope(
            command.registry_generation(),
            &command.request_id,
            &command.page_id,
            &command.apply_unit_id,
            &command.scope_id,
        )
    }

    pub(super) fn reject_reset_envelope(
        &self,
        command: &SettingsResetCommand,
    ) -> Option<SettingsMutationResult> {
        if !self.definition.reset_supported {
            return Some(SettingsMutationResult::Rejected {
                rejection: rejection(SettingsRejectionCode::InvalidIntent),
                snapshot: None,
            });
        }
        self.reject_envelope(
            command.registry_generation(),
            &command.request_id,
            &command.page_id,
            &command.apply_unit_id,
            &command.scope_id,
        )
    }

    fn reject_envelope(
        &self,
        generation: longhorn_settings::SettingsRegistryGeneration,
        _request_id: &SettingsRequestId,
        page_id: &longhorn_core::SettingsPageId,
        apply_unit_id: &longhorn_core::SettingsApplyUnitId,
        scope_id: &longhorn_core::SettingsScopeId,
    ) -> Option<SettingsMutationResult> {
        let code = if generation != self.generation
            || apply_unit_id != &self.definition.id
            || scope_id != &self.definition.scope_id
        {
            Some(SettingsRejectionCode::RegistryChanged)
        } else if !self.authorized_pages.contains(page_id) {
            Some(SettingsRejectionCode::Unauthorized)
        } else {
            None
        };
        code.map(|code| SettingsMutationResult::Rejected {
            rejection: rejection(code),
            snapshot: None,
        })
    }
}

trait CommandGeneration {
    fn registry_generation(&self) -> longhorn_settings::SettingsRegistryGeneration;
}

impl CommandGeneration for SettingsApplyCommand {
    fn registry_generation(&self) -> longhorn_settings::SettingsRegistryGeneration {
        self.authority.registry_generation
    }
}

impl CommandGeneration for SettingsResetCommand {
    fn registry_generation(&self) -> longhorn_settings::SettingsRegistryGeneration {
        self.authority.registry_generation
    }
}

pub(super) fn rejection(code: SettingsRejectionCode) -> SettingsRejection {
    SettingsRejection {
        code,
        diagnostic: None,
    }
}
