use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::ids::{AdapterDisplayKey, StrongDisplayKey, WeakDisplayKey};

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
