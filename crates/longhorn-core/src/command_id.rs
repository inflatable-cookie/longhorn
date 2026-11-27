use crate::opaque_id::opaque_id;

opaque_id!(CommandId, "Stable semantic identity for one command.");
opaque_id!(
    CommandContextId,
    "Consumer-owned identity for one command context."
);
opaque_id!(
    CommandCategoryId,
    "Stable identity for one command discovery category."
);
opaque_id!(
    CommandRouteId,
    "Opaque consumer route identity for one command."
);
opaque_id!(
    CommandCapabilityId,
    "Stable identity for one command composition capability."
);
opaque_id!(
    CommandFieldId,
    "Stable identity for one command argument field."
);
opaque_id!(
    CommandEnumValueId,
    "Stable identity for one closed command enum value."
);
opaque_id!(
    CommandRequestId,
    "Stable correlation identity for one command execution request."
);
opaque_id!(
    CommandAvailabilityReasonId,
    "Consumer-owned command availability reason identity."
);
opaque_id!(
    CommandEvidenceCode,
    "Consumer-owned command outcome evidence code."
);
opaque_id!(
    CommandKeymapPresetId,
    "Stable identity for one immutable command keymap preset."
);
opaque_id!(
    CommandBindingId,
    "Stable identity for one base or added command binding."
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opaque_id::OpaqueIdError;

    #[test]
    fn command_ids_share_the_bounded_core_grammar() {
        assert!(CommandId::new("loophole:transport.play").is_ok());
        assert!(CommandContextId::new("global").is_ok());
        assert!(CommandRouteId::new("consumer:transport.play").is_ok());
        assert!(CommandRequestId::new("request:0198f97e").is_ok());
        assert!(CommandAvailabilityReasonId::new("editor:no-selection").is_ok());
        assert!(CommandKeymapPresetId::new("loophole:default").is_ok());
        assert!(CommandBindingId::new("transport:play").is_ok());
        assert_eq!(
            CommandFieldId::new("TrackName"),
            Err(OpaqueIdError::InvalidCharacter { index: 0 })
        );
    }
}
