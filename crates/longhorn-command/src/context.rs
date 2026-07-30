use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_core::{CommandCapabilityId, CommandContextId};
use serde::{Deserialize, Deserializer, Serialize, de};

const HARD_MAXIMUM_CONTEXT_PATH: usize = 256;
const HARD_MAXIMUM_CAPABILITY_FACTS: usize = 4_096;

/// Monotonic identity of one consumer-owned current context snapshot.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct CommandContextRevision(u64);

impl CommandContextRevision {
    /// Initial consumer context revision.
    pub const INITIAL: Self = Self(0);

    /// Constructs a revision from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next revision without wrapping.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// One ordered current context path from `global` to the hottest leaf.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandContextSnapshot {
    revision: CommandContextRevision,
    path: Vec<CommandContextId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CommandContextSnapshotWire {
    revision: CommandContextRevision,
    path: Vec<CommandContextId>,
}

impl<'de> Deserialize<'de> for CommandContextSnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CommandContextSnapshotWire::deserialize(deserializer)?;
        Self::new(wire.revision, wire.path).map_err(de::Error::custom)
    }
}

impl CommandContextSnapshot {
    /// Constructs a locally well-formed bounded hot-context path.
    pub fn new(
        revision: CommandContextRevision,
        path: Vec<CommandContextId>,
    ) -> Result<Self, CommandContextSnapshotError> {
        if path.is_empty() {
            return Err(CommandContextSnapshotError::EmptyPath);
        }
        if path.len() > HARD_MAXIMUM_CONTEXT_PATH {
            return Err(CommandContextSnapshotError::PathTooDeep {
                maximum: HARD_MAXIMUM_CONTEXT_PATH,
                actual: path.len(),
            });
        }
        if path[0].as_str() != "global" {
            return Err(CommandContextSnapshotError::MissingGlobalRoot);
        }
        let unique: BTreeSet<_> = path.iter().collect();
        if unique.len() != path.len() {
            return Err(CommandContextSnapshotError::DuplicateContext);
        }
        Ok(Self { revision, path })
    }

    /// Returns the consumer context revision.
    #[must_use]
    pub const fn revision(&self) -> CommandContextRevision {
        self.revision
    }

    /// Returns the ordered root-to-leaf context path.
    pub fn path(&self) -> impl ExactSizeIterator<Item = &CommandContextId> {
        self.path.iter()
    }

    pub(crate) fn path_slice(&self) -> &[CommandContextId] {
        &self.path
    }
}

/// Invalid locally constructed current context path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandContextSnapshotError {
    /// The path contains no context.
    EmptyPath,
    /// The first path entry is not `global`.
    MissingGlobalRoot,
    /// One context appears more than once.
    DuplicateContext,
    /// The path exceeds the defensive hard ceiling.
    PathTooDeep {
        /// Maximum admitted entries.
        maximum: usize,
        /// Supplied entries.
        actual: usize,
    },
}

impl fmt::Display for CommandContextSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("command context path is empty"),
            Self::MissingGlobalRoot => {
                formatter.write_str("command context path must begin with global")
            }
            Self::DuplicateContext => {
                formatter.write_str("command context path contains a duplicate context")
            }
            Self::PathTooDeep { maximum, actual } => write!(
                formatter,
                "command context path has {actual} entries; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for CommandContextSnapshotError {}

/// Canonical current command capability facts supplied by the consumer.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CommandCapabilitySnapshot(BTreeSet<CommandCapabilityId>);

impl CommandCapabilitySnapshot {
    /// Constructs a bounded, deduplicated capability set.
    pub fn new(
        capabilities: impl IntoIterator<Item = CommandCapabilityId>,
    ) -> Result<Self, CommandCapabilitySnapshotError> {
        let mut values = BTreeSet::new();
        for (index, capability) in capabilities.into_iter().enumerate() {
            if index >= HARD_MAXIMUM_CAPABILITY_FACTS {
                return Err(CommandCapabilitySnapshotError::TooManyCapabilities {
                    maximum: HARD_MAXIMUM_CAPABILITY_FACTS,
                });
            }
            values.insert(capability);
        }
        Ok(Self(values))
    }

    /// Returns current capability ids in stable order.
    pub fn capabilities(&self) -> impl ExactSizeIterator<Item = &CommandCapabilityId> {
        self.0.iter()
    }

    /// Returns whether one capability is currently present.
    #[must_use]
    pub fn contains(&self, capability_id: &CommandCapabilityId) -> bool {
        self.0.contains(capability_id)
    }
}

impl<'de> Deserialize<'de> for CommandCapabilitySnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(Vec::<CommandCapabilityId>::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Invalid current command capability facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandCapabilitySnapshotError {
    /// The set exceeds the defensive hard ceiling.
    TooManyCapabilities {
        /// Maximum admitted distinct capabilities.
        maximum: usize,
    },
}

impl fmt::Display for CommandCapabilitySnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyCapabilities { maximum } => write!(
                formatter,
                "command capability snapshot exceeds hard maximum {maximum}"
            ),
        }
    }
}

impl Error for CommandCapabilitySnapshotError {}
