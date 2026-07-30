use longhorn_core::{
    SettingsActivationTargetId, SettingsAnchorId, SettingsApplyUnitId, SettingsCapabilityId,
    SettingsEntryId, SettingsModuleId, SettingsPageId, SettingsPolicySourceId, SettingsRendererId,
    SettingsRequestId, SettingsScopeId, SettingsSectionId,
};

macro_rules! id {
    ($function:ident, $type:ty) => {
        pub(super) fn $function(value: &str) -> $type {
            <$type>::new(value).unwrap()
        }
    };
}

id!(module_id, SettingsModuleId);
id!(section_id, SettingsSectionId);
id!(page_id, SettingsPageId);
id!(renderer_id, SettingsRendererId);
id!(anchor_id, SettingsAnchorId);
id!(scope_id, SettingsScopeId);
id!(unit_id, SettingsApplyUnitId);
id!(capability_id, SettingsCapabilityId);
id!(activation_id, SettingsActivationTargetId);
id!(entry_id, SettingsEntryId);
id!(request_id, SettingsRequestId);
id!(policy_id, SettingsPolicySourceId);
