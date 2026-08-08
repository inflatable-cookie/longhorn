use std::{collections::BTreeMap, error::Error, fmt};

use longhorn_core::DisplayId;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use super::displays::KnownDisplay;

/// Persistent known-display collection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KnownDisplayRegistry {
    displays: BTreeMap<DisplayId, KnownDisplay>,
}

impl KnownDisplayRegistry {
    /// Constructs an empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            displays: BTreeMap::new(),
        }
    }

    /// Validates and constructs a registry.
    pub fn from_displays(
        displays: impl IntoIterator<Item = KnownDisplay>,
    ) -> Result<Self, RegistryError> {
        let mut registry = Self::new();
        for display in displays {
            let id = display.id().clone();
            if registry.displays.insert(id.clone(), display).is_some() {
                return Err(RegistryError::DuplicateDisplayId(id));
            }
        }
        Ok(registry)
    }

    /// Returns a known display.
    #[must_use]
    pub fn get(&self, id: &DisplayId) -> Option<&KnownDisplay> {
        self.displays.get(id)
    }

    /// Returns known displays in canonical-id order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &KnownDisplay> {
        self.displays.values()
    }

    /// Returns the number of known displays.
    #[must_use]
    pub fn len(&self) -> usize {
        self.displays.len()
    }

    /// Returns whether no displays are known.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.displays.is_empty()
    }

    /// Explicitly forgets one named known display.
    pub fn forget(&mut self, id: &DisplayId) -> Option<KnownDisplay> {
        self.displays.remove(id)
    }

    pub(crate) fn insert(&mut self, display: KnownDisplay) -> Option<KnownDisplay> {
        self.displays.insert(display.id().clone(), display)
    }

    pub(crate) fn get_mut(&mut self, id: &DisplayId) -> Option<&mut KnownDisplay> {
        self.displays.get_mut(id)
    }
}

impl Serialize for KnownDisplayRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.displays
            .values()
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for KnownDisplayRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let displays = Vec::<KnownDisplay>::deserialize(deserializer)?;
        Self::from_displays(displays).map_err(de::Error::custom)
    }
}

/// Known-display registry validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    /// Two records declared the same canonical identity.
    DuplicateDisplayId(DisplayId),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDisplayId(id) => {
                write!(
                    formatter,
                    "known display registry contains duplicate id {id}"
                )
            }
        }
    }
}

impl Error for RegistryError {}
