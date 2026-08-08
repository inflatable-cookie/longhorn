use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    DisplayTextError,
    text::{
        validate_evidence_namespace, validate_evidence_value, validate_label,
        validate_observation_id,
    },
};

macro_rules! string_value {
    ($name:ident, $description:literal, $validator:ident) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Validates and constructs the value.
            pub fn new(value: impl Into<String>) -> Result<Self, DisplayTextError> {
                let value = value.into();
                $validator(&value)?;
                Ok(Self(value))
            }

            /// Returns the serialized value.
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

string_value!(
    ObservationId,
    "Ephemeral identifier for one current host observation.",
    validate_observation_id
);
string_value!(
    DisplayLabel,
    "Bounded machine or user display label.",
    validate_label
);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct EvidenceKey {
    namespace: String,
    value: String,
}

impl EvidenceKey {
    fn new(
        namespace: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, DisplayTextError> {
        let namespace = namespace.into();
        let value = value.into();
        validate_evidence_namespace(&namespace)?;
        validate_evidence_value(&value)?;
        Ok(Self { namespace, value })
    }
}

impl<'de> Deserialize<'de> for EvidenceKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            namespace: String,
            value: String,
        }

        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.namespace, wire.value).map_err(de::Error::custom)
    }
}

macro_rules! evidence_key {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(EvidenceKey);

        impl $name {
            /// Validates and constructs a namespaced evidence key.
            pub fn new(
                namespace: impl Into<String>,
                value: impl Into<String>,
            ) -> Result<Self, DisplayTextError> {
                Ok(Self(EvidenceKey::new(namespace, value)?))
            }

            /// Returns the evidence namespace.
            #[must_use]
            pub fn namespace(&self) -> &str {
                &self.0.namespace
            }

            /// Returns the opaque evidence value.
            #[must_use]
            pub fn value(&self) -> &str {
                &self.0.value
            }
        }
    };
}

evidence_key!(
    StrongDisplayKey,
    "Namespaced platform or hardware evidence with strong identity semantics."
);
evidence_key!(
    AdapterDisplayKey,
    "Namespaced process or host-adapter display evidence."
);
evidence_key!(
    WeakDisplayKey,
    "Namespaced weak fingerprint that cannot establish identity when duplicated."
);
