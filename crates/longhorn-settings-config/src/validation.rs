use longhorn_config::ConfigDomain;
use longhorn_core::{SettingsApplyUnitId, SettingsPageId, SettingsRequestId, SettingsScopeId};
use longhorn_settings::{
    SettingsAuthorityExpectation, SettingsMutationResult, SettingsRejection, SettingsRejectionCode,
};

use crate::{ConfigSettingsApplyUnit, SettingsConfigAdapter};

pub(super) struct MutationEnvelope<'command> {
    pub(super) request_id: &'command SettingsRequestId,
    pub(super) page_id: &'command SettingsPageId,
    pub(super) apply_unit_id: &'command SettingsApplyUnitId,
    pub(super) scope_id: &'command SettingsScopeId,
    pub(super) authority: &'command SettingsAuthorityExpectation,
}

impl<D, A> ConfigSettingsApplyUnit<D, A>
where
    D: ConfigDomain,
    A: SettingsConfigAdapter<D::Value>,
{
    pub(super) fn reject_envelope(
        &self,
        envelope: &MutationEnvelope<'_>,
        reset: bool,
    ) -> Option<SettingsMutationResult> {
        let code = if envelope.authority.registry_generation != self.generation
            || envelope.apply_unit_id != &self.definition.id
            || envelope.scope_id != &self.definition.scope_id
        {
            SettingsRejectionCode::RegistryChanged
        } else {
            match self.authorized_pages.get(envelope.page_id) {
                None => SettingsRejectionCode::Unauthorized,
                Some(features) if reset && !features.reset => SettingsRejectionCode::Unauthorized,
                Some(_) if reset && !self.definition.reset_supported => {
                    SettingsRejectionCode::Unsupported
                }
                Some(_) => return None,
            }
        };
        Some(SettingsMutationResult::Rejected {
            rejection: rejection(code),
            snapshot: None,
        })
    }
}

pub(super) fn rejection(code: SettingsRejectionCode) -> SettingsRejection {
    SettingsRejection {
        code,
        diagnostic: None,
    }
}
