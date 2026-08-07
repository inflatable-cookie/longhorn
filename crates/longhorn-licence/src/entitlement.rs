use core::fmt;
use std::{collections::BTreeMap, error::Error};

use serde::{Deserialize, Serialize};

/// Longest accepted entitlement identifier.
const MAXIMUM_ENTITLEMENT_BYTES: usize = 128;

/// A consumer-defined capability identifier.
///
/// Opaque to Longhorn by design. Longhorn evaluates presence and limits and
/// never enumerates, interprets, or reserves an identifier — modelling a
/// fixed edition enum instead would make every pricing change a code change
/// and a release.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntitlementId(String);

impl From<EntitlementId> for String {
    fn from(value: EntitlementId) -> Self {
        value.0
    }
}

impl EntitlementId {
    /// Validates and records an identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, EntitlementIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(EntitlementIdError::Empty);
        }
        if value.len() > MAXIMUM_ENTITLEMENT_BYTES {
            return Err(EntitlementIdError::TooLong {
                maximum: MAXIMUM_ENTITLEMENT_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EntitlementId {
    type Error = EntitlementIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for EntitlementId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Entitlement identifier validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntitlementIdError {
    /// The identifier was empty.
    Empty,
    /// The identifier exceeded its bound.
    TooLong {
        /// Accepted maximum.
        maximum: usize,
        /// Supplied length.
        actual: usize,
    },
}

impl fmt::Display for EntitlementIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("entitlement id must not be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "entitlement id is {actual} bytes; maximum is {maximum}"
            ),
        }
    }
}

impl Error for EntitlementIdError {}

/// A bound on an entitlement, where one applies.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum Limit {
    /// No bound.
    #[default]
    Unlimited,
    /// At most this many.
    AtMost(u64),
}

impl Limit {
    /// Returns whether a requested count is permitted.
    #[must_use]
    pub const fn permits(self, requested: u64) -> bool {
        match self {
            Self::Unlimited => true,
            Self::AtMost(maximum) => requested <= maximum,
        }
    }

    /// Returns the bound, if there is one.
    #[must_use]
    pub const fn maximum(self) -> Option<u64> {
        match self {
            Self::Unlimited => None,
            Self::AtMost(maximum) => Some(maximum),
        }
    }
}

impl fmt::Display for Limit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unlimited => formatter.write_str("unlimited"),
            Self::AtMost(maximum) => write!(formatter, "at most {maximum}"),
        }
    }
}

/// The set of capabilities a licence grants.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Entitlements(BTreeMap<EntitlementId, Limit>);

impl Entitlements {
    /// Records an empty grant.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants one entitlement with a bound.
    #[must_use]
    pub fn with(mut self, id: EntitlementId, limit: Limit) -> Self {
        self.0.insert(id, limit);
        self
    }

    /// Grants one unbounded entitlement.
    #[must_use]
    pub fn granting(self, id: EntitlementId) -> Self {
        self.with(id, Limit::Unlimited)
    }

    /// Returns whether an entitlement is granted at all.
    #[must_use]
    pub fn grants(&self, id: &EntitlementId) -> bool {
        self.0.contains_key(id)
    }

    /// Returns the bound on an entitlement, or `None` when it is not granted.
    ///
    /// Absent and unlimited are distinct answers, so a caller cannot mistake
    /// "not sold this" for "sold this without a cap".
    #[must_use]
    pub fn limit(&self, id: &EntitlementId) -> Option<Limit> {
        self.0.get(id).copied()
    }

    /// Returns whether an entitlement is granted and permits a count.
    #[must_use]
    pub fn permits(&self, id: &EntitlementId, requested: u64) -> bool {
        self.limit(id).is_some_and(|limit| limit.permits(requested))
    }

    /// Returns every granted entitlement.
    pub fn iter(&self) -> impl Iterator<Item = (&EntitlementId, Limit)> {
        self.0.iter().map(|(id, limit)| (id, *limit))
    }

    /// Returns how many entitlements are granted.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether nothing is granted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> EntitlementId {
        EntitlementId::new(value).unwrap()
    }

    #[test]
    fn absent_and_unlimited_are_distinguishable() {
        let entitlements = Entitlements::new().granting(id("surfaces.multi-window"));

        assert_eq!(
            entitlements.limit(&id("surfaces.multi-window")),
            Some(Limit::Unlimited)
        );
        assert_eq!(entitlements.limit(&id("history.fork-tree")), None);
    }

    #[test]
    fn an_absent_entitlement_permits_nothing() {
        let entitlements = Entitlements::new();

        assert!(!entitlements.permits(&id("seats"), 0));
        assert!(!entitlements.grants(&id("seats")));
    }

    #[test]
    fn a_bound_is_inclusive() {
        let entitlements = Entitlements::new().with(id("seats"), Limit::AtMost(5));

        assert!(entitlements.permits(&id("seats"), 5));
        assert!(!entitlements.permits(&id("seats"), 6));
    }

    #[test]
    fn identifiers_are_bounded_and_non_empty() {
        assert_eq!(EntitlementId::new(""), Err(EntitlementIdError::Empty));
        assert!(matches!(
            EntitlementId::new("x".repeat(MAXIMUM_ENTITLEMENT_BYTES + 1)),
            Err(EntitlementIdError::TooLong { .. })
        ));
    }

    #[test]
    fn longhorn_reserves_no_identifier() {
        // Any string a consumer chooses is acceptable, including ones that
        // look like they might mean something to Longhorn.
        for candidate in ["licensed", "longhorn.core", "*", "edition:pro", "🎟"] {
            assert!(EntitlementId::new(candidate).is_ok(), "{candidate}");
        }
    }
}
