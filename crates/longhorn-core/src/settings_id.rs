use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::opaque_id::{OpaqueIdError, validate_opaque_id};

macro_rules! settings_id {
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

settings_id!(
    SettingsModuleId,
    "Opaque identity for one settings registration module."
);
settings_id!(
    SettingsSectionId,
    "Opaque identity for one settings navigation section."
);
settings_id!(
    SettingsPageId,
    "Opaque identity for one registered settings page."
);
settings_id!(
    SettingsRendererId,
    "Opaque resolver identity for one settings page renderer."
);
settings_id!(
    SettingsAnchorId,
    "Opaque identity for one stable settings page anchor."
);
settings_id!(
    SettingsScopeId,
    "Opaque identity for one authoritative settings value scope."
);
settings_id!(
    SettingsApplyUnitId,
    "Opaque identity for one failure-atomic settings mutation unit."
);
settings_id!(
    SettingsCapabilityId,
    "Opaque identity for one settings composition capability."
);
settings_id!(
    SettingsActivationTargetId,
    "Opaque identity for one runtime settings activation target."
);
settings_id!(
    SettingsEntryId,
    "Opaque consumer-owned identity for one projected settings value."
);
settings_id!(
    SettingsRequestId,
    "Opaque identity for one settings protocol request."
);
settings_id!(
    SettingsPolicySourceId,
    "Opaque identity for one settings policy provenance source."
);
settings_id!(
    SettingsAuthorityToken,
    "Opaque host-issued token for one authoritative settings scope snapshot."
);

#[cfg(test)]
mod tests {
    use super::*;

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
