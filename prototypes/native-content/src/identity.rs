use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const MAX_ID_BYTES: usize = 128;

macro_rules! bounded_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Validates and constructs the identifier.
            pub fn new(value: impl Into<String>) -> Result<Self, NativeContentIdError> {
                let value = value.into();
                validate_id(&value)?;
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
            type Err = NativeContentIdError;

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

bounded_id!(
    NativeContentIslandId,
    "Opaque identity for one native content island."
);
bounded_id!(
    NativeContentKindId,
    "Bounded consumer-owned kind identity for one native content island."
);
bounded_id!(
    VisibilityReasonId,
    "Bounded consumer-owned reason for desired hidden presentation."
);
bounded_id!(
    NativeContentFailureCode,
    "Bounded stable code for a mechanism operation failure."
);

fn validate_id(value: &str) -> Result<(), NativeContentIdError> {
    if value.is_empty() {
        return Err(NativeContentIdError::Empty);
    }
    if value.len() > MAX_ID_BYTES {
        return Err(NativeContentIdError::TooLong {
            maximum: MAX_ID_BYTES,
            actual: value.len(),
        });
    }
    if let Some((index, _)) = value.char_indices().find(|(_, character)| {
        !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | ':' | '-'))
    }) {
        return Err(NativeContentIdError::InvalidCharacter { index });
    }
    Ok(())
}

/// Validation failure for a private native-content identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeContentIdError {
    /// The identifier was empty.
    Empty,
    /// The identifier exceeded the bounded serialized length.
    TooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// A character fell outside the stable lowercase ASCII grammar.
    InvalidCharacter {
        /// Byte index of the rejected character.
        index: usize,
    },
}

impl fmt::Display for NativeContentIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("native-content id cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "native-content id is {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidCharacter { index } => {
                write!(formatter, "native-content id has invalid byte at {index}")
            }
        }
    }
}

impl Error for NativeContentIdError {}

macro_rules! monotonic_counter {
    ($name:ident, $initial:expr, $description:literal, $overflow:literal) => {
        #[doc = $description]
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            /// Initial value for a new coordination authority.
            pub const INITIAL: Self = Self($initial);

            /// Constructs a value from its serialized representation.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the serialized value.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }

            /// Advances the value or fails instead of wrapping.
            pub const fn checked_next(self) -> Result<Self, CounterOverflow> {
                match self.0.checked_add(1) {
                    Some(value) => Ok(Self(value)),
                    None => Err(CounterOverflow($overflow)),
                }
            }
        }
    };
}

monotonic_counter!(
    NativeContentRevision,
    0,
    "Monotonic revision of desired or observed native-content state.",
    "native-content revision overflow"
);
monotonic_counter!(
    AttachGeneration,
    1,
    "Monotonic identity for one native-content attach attempt.",
    "attach generation overflow"
);

/// A native-content counter could not advance without wrapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterOverflow(&'static str);

impl fmt::Display for CounterOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for CounterOverflow {}

/// Ordered identity for one operation in an apply plan.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PlanStepId(u32);

impl PlanStepId {
    /// Constructs a step id. Plan construction starts at one.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the serialized step number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
