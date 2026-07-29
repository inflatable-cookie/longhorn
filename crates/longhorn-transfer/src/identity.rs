use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Exact 128-bit process-local transfer-session identity.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DragSessionId([u8; 16]);

impl DragSessionId {
    /// Constructs an id from allocator-supplied entropy.
    #[must_use]
    pub const fn from_entropy(entropy: [u8; 16]) -> Self {
        Self(entropy)
    }

    /// Returns the exact allocator-supplied entropy.
    #[must_use]
    pub const fn entropy(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Debug for DragSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for DragSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for DragSessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DragSessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_session_id(&value).map_err(de::Error::custom)
    }
}

fn parse_session_id(value: &str) -> Result<DragSessionId, DragSessionIdParseError> {
    if value.len() != 32 {
        return Err(DragSessionIdParseError::InvalidLength {
            actual: value.len(),
        });
    }
    let mut entropy = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        entropy[index] = parse_hex(pair[0])
            .and_then(|high| parse_hex(pair[1]).map(|low| high << 4 | low))
            .ok_or(DragSessionIdParseError::InvalidHex { index: index * 2 })?;
    }
    Ok(DragSessionId::from_entropy(entropy))
}

fn parse_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// Rejected serialized transfer-session identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DragSessionIdParseError {
    /// The id was not exactly 32 lowercase hexadecimal bytes.
    InvalidLength {
        /// Observed serialized byte length.
        actual: usize,
    },
    /// The id contained a non-lowercase-hexadecimal byte.
    InvalidHex {
        /// First byte index of the rejected pair.
        index: usize,
    },
}

impl fmt::Display for DragSessionIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { actual } => {
                write!(formatter, "drag session id is {actual} bytes; expected 32")
            }
            Self::InvalidHex { index } => {
                write!(formatter, "drag session id has invalid hex at byte {index}")
            }
        }
    }
}

impl Error for DragSessionIdParseError {}

/// Allocator failure before a transfer session receives identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DragSessionIdAllocationError;

impl fmt::Display for DragSessionIdAllocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("drag session entropy allocation failed")
    }
}

impl Error for DragSessionIdAllocationError {}

/// Injected cryptographically strong 128-bit session-id allocator.
pub trait DragSessionIdAllocator {
    /// Allocates fresh entropy or returns an explicit failure.
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError>;
}

macro_rules! unsigned_identity {
    ($name:ident, $inner:ty, $description:literal) => {
        #[doc = $description]
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name($inner);

        impl $name {
            /// Constructs the value.
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            /// Returns the stored value.
            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }
    };
}

unsigned_identity!(
    TransferInstant,
    u64,
    "Injected monotonic-clock instant in host-selected ticks."
);
unsigned_identity!(
    TransferDuration,
    u64,
    "Finite transfer lifetime in the same ticks as `TransferInstant`."
);
unsigned_identity!(
    ClientEpoch,
    u64,
    "Host-issued renderer-client epoch for one managed window."
);
unsigned_identity!(
    LeaseGeneration,
    u64,
    "Monotonic complete-replacement generation inside one client epoch."
);
unsigned_identity!(
    TransferRevision,
    u64,
    "Adapter-projected authoritative document revision."
);
unsigned_identity!(
    InsertionPosition,
    u32,
    "Advisory bounded insertion position inside one target region."
);

/// Injected monotonic time source.
pub trait MonotonicClock {
    /// Returns the current process-local transfer instant.
    fn now(&self) -> TransferInstant;
}
