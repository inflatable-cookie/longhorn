use crate::opaque_id::opaque_id;

opaque_id!(
    SettingsModuleId,
    "Opaque identity for one settings registration module."
);
opaque_id!(
    SettingsSectionId,
    "Opaque identity for one settings navigation section."
);
opaque_id!(
    SettingsPageId,
    "Opaque identity for one registered settings page."
);
opaque_id!(
    SettingsRendererId,
    "Opaque resolver identity for one settings page renderer."
);
opaque_id!(
    SettingsAnchorId,
    "Opaque identity for one stable settings page anchor."
);
opaque_id!(
    SettingsScopeId,
    "Opaque identity for one authoritative settings value scope."
);
opaque_id!(
    SettingsApplyUnitId,
    "Opaque identity for one failure-atomic settings mutation unit."
);
opaque_id!(
    SettingsCapabilityId,
    "Opaque identity for one settings composition capability."
);
opaque_id!(
    SettingsActivationTargetId,
    "Opaque identity for one runtime settings activation target."
);
opaque_id!(
    SettingsEntryId,
    "Opaque consumer-owned identity for one projected settings value."
);
opaque_id!(
    SettingsRequestId,
    "Opaque identity for one settings protocol request."
);
opaque_id!(
    SettingsPolicySourceId,
    "Opaque identity for one settings policy provenance source."
);
opaque_id!(
    SettingsAuthorityToken,
    "Opaque host-issued token for one authoritative settings scope snapshot."
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opaque_id::OpaqueIdError;

    #[test]
    fn settings_ids_share_the_bounded_core_grammar() {
        assert!(SettingsModuleId::new("soundcheck:preferences").is_ok());
        assert!(SettingsPageId::new("soundcheck:audio").is_ok());
        assert!(SettingsAuthorityToken::new("token:0198f97e").is_ok());
        assert_eq!(
            SettingsRendererId::new("ConsumerRenderer"),
            Err(OpaqueIdError::InvalidCharacter { index: 0 })
        );
    }
}
