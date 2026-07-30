use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::opaque_id::{OpaqueIdError, validate_opaque_id};

macro_rules! command_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
        #[cfg_attr(feature = "bindings", ts(type = "string"))]
        pub struct $name(String);

        impl $name {
            /// Validates and constructs the identifier.
            pub fn new(value: impl Into<String>) -> Result<Self, OpaqueIdError> {
                let value = value.into();
                validate_opaque_id(&value)?;
                Ok(Self(value))
            }

            /// Returns the serialized identifier.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = OpaqueIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(de::Error::custom)
            }
        }
    };
}

command_id!(CommandId, "Stable semantic identity for one command.");
command_id!(
    CommandContextId,
    "Consumer-owned identity for one command context."
);
command_id!(
    CommandCategoryId,
    "Stable identity for one command discovery category."
);
command_id!(
    CommandRouteId,
    "Opaque consumer route identity for one command."
);
command_id!(
    CommandCapabilityId,
    "Stable identity for one command composition capability."
);
command_id!(
    CommandFieldId,
    "Stable identity for one command argument field."
);
command_id!(
    CommandEnumValueId,
    "Stable identity for one closed command enum value."
);
command_id!(
    CommandRequestId,
    "Stable correlation identity for one command execution request."
);
command_id!(
    CommandAvailabilityReasonId,
    "Consumer-owned command availability reason identity."
);
command_id!(
    CommandEvidenceCode,
    "Consumer-owned command outcome evidence code."
);
command_id!(
    CommandKeymapPresetId,
    "Stable identity for one immutable command keymap preset."
);
command_id!(
    CommandBindingId,
    "Stable identity for one base or added command binding."
);

#[cfg(test)]
mod tests {
    use super::*;

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
