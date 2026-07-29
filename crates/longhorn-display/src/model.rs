use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use longhorn_core::{DisplayId, ScaleFactor, ScreenRect};
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

/// Host knowledge about whether a display is physically built in.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayBuiltinStatus {
    /// The host adapter cannot determine built-in status.
    Unknown,
    /// The host identifies the display as built in.
    BuiltIn,
    /// The host identifies the display as external.
    External,
}

/// Current display facts expressed in screen DIPs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DisplayFacts {
    machine_label: Option<DisplayLabel>,
    is_main: bool,
    builtin_status: DisplayBuiltinStatus,
    full_bounds: ScreenRect,
    work_area: ScreenRect,
    scale: ScaleFactor,
}

impl DisplayFacts {
    /// Constructs current or last-observed display facts.
    #[must_use]
    pub const fn new(
        machine_label: Option<DisplayLabel>,
        is_main: bool,
        builtin_status: DisplayBuiltinStatus,
        full_bounds: ScreenRect,
        work_area: ScreenRect,
        scale: ScaleFactor,
    ) -> Self {
        Self {
            machine_label,
            is_main,
            builtin_status,
            full_bounds,
            work_area,
            scale,
        }
    }

    /// Returns the machine-provided label.
    #[must_use]
    pub const fn machine_label(&self) -> Option<&DisplayLabel> {
        self.machine_label.as_ref()
    }

    /// Returns whether the host currently marks this as the main display.
    #[must_use]
    pub const fn is_main(&self) -> bool {
        self.is_main
    }

    /// Returns the host's current built-in status.
    #[must_use]
    pub const fn builtin_status(&self) -> DisplayBuiltinStatus {
        self.builtin_status
    }

    /// Returns full display bounds.
    #[must_use]
    pub const fn full_bounds(&self) -> ScreenRect {
        self.full_bounds
    }

    /// Returns usable work-area bounds.
    #[must_use]
    pub const fn work_area(&self) -> ScreenRect {
        self.work_area
    }

    /// Returns current scale evidence.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }
}

/// Correlation evidence retained for a known or observed display.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DisplayEvidence {
    strong_keys: BTreeSet<StrongDisplayKey>,
    adapter_keys: BTreeSet<AdapterDisplayKey>,
    weak_keys: BTreeSet<WeakDisplayKey>,
}

impl DisplayEvidence {
    /// Constructs empty evidence.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strong_keys: BTreeSet::new(),
            adapter_keys: BTreeSet::new(),
            weak_keys: BTreeSet::new(),
        }
    }

    /// Adds strong platform or hardware evidence.
    #[must_use]
    pub fn with_strong_key(mut self, key: StrongDisplayKey) -> Self {
        self.strong_keys.insert(key);
        self
    }

    /// Adds host-adapter evidence.
    #[must_use]
    pub fn with_adapter_key(mut self, key: AdapterDisplayKey) -> Self {
        self.adapter_keys.insert(key);
        self
    }

    /// Adds a weak fingerprint.
    #[must_use]
    pub fn with_weak_key(mut self, key: WeakDisplayKey) -> Self {
        self.weak_keys.insert(key);
        self
    }

    /// Returns strong evidence in canonical order.
    #[must_use]
    pub const fn strong_keys(&self) -> &BTreeSet<StrongDisplayKey> {
        &self.strong_keys
    }

    /// Returns adapter evidence in canonical order.
    #[must_use]
    pub const fn adapter_keys(&self) -> &BTreeSet<AdapterDisplayKey> {
        &self.adapter_keys
    }

    /// Returns weak evidence in canonical order.
    #[must_use]
    pub const fn weak_keys(&self) -> &BTreeSet<WeakDisplayKey> {
        &self.weak_keys
    }

    pub(crate) fn merge(&mut self, other: &Self) {
        self.strong_keys.extend(other.strong_keys.iter().cloned());
        self.adapter_keys.extend(other.adapter_keys.iter().cloned());
        self.weak_keys.extend(other.weak_keys.iter().cloned());
    }
}

/// A display retained across observation cycles.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnownDisplay {
    id: DisplayId,
    facts: DisplayFacts,
    user_label: Option<DisplayLabel>,
    evidence: DisplayEvidence,
}

impl KnownDisplay {
    /// Constructs a known display from allocated identity and observed facts.
    #[must_use]
    pub const fn new(id: DisplayId, facts: DisplayFacts, evidence: DisplayEvidence) -> Self {
        Self {
            id,
            facts,
            user_label: None,
            evidence,
        }
    }

    /// Returns canonical machine-local identity.
    #[must_use]
    pub const fn id(&self) -> &DisplayId {
        &self.id
    }

    /// Returns last-observed facts.
    #[must_use]
    pub const fn facts(&self) -> &DisplayFacts {
        &self.facts
    }

    /// Returns retained correlation evidence.
    #[must_use]
    pub const fn evidence(&self) -> &DisplayEvidence {
        &self.evidence
    }

    /// Returns the explicit user label.
    #[must_use]
    pub const fn user_label(&self) -> Option<&DisplayLabel> {
        self.user_label.as_ref()
    }

    /// Returns the user label when present, otherwise the machine label.
    #[must_use]
    pub fn effective_label(&self) -> Option<&DisplayLabel> {
        self.user_label
            .as_ref()
            .or_else(|| self.facts.machine_label())
    }

    /// Sets or clears the user label without erasing the machine label.
    pub fn set_user_label(&mut self, label: Option<DisplayLabel>) {
        self.user_label = label;
    }

    pub(crate) fn observe(&mut self, observation: &ObservedDisplay) {
        self.facts = observation.facts.clone();
        self.evidence.merge(&observation.evidence);
    }
}

/// One current host display observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedDisplay {
    observation_id: ObservationId,
    facts: DisplayFacts,
    evidence: DisplayEvidence,
}

impl ObservedDisplay {
    /// Constructs a host observation without assigning canonical identity.
    #[must_use]
    pub const fn new(
        observation_id: ObservationId,
        facts: DisplayFacts,
        evidence: DisplayEvidence,
    ) -> Self {
        Self {
            observation_id,
            facts,
            evidence,
        }
    }

    /// Returns ephemeral observation identity.
    #[must_use]
    pub const fn observation_id(&self) -> &ObservationId {
        &self.observation_id
    }

    /// Returns observed facts.
    #[must_use]
    pub const fn facts(&self) -> &DisplayFacts {
        &self.facts
    }

    /// Returns observed correlation evidence.
    #[must_use]
    pub const fn evidence(&self) -> &DisplayEvidence {
        &self.evidence
    }
}

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
            let id = display.id.clone();
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
        self.displays.insert(display.id.clone(), display)
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
