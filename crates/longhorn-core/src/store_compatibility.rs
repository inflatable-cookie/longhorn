use std::fmt;

/// Which persistent store refused a load.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum CompatibilityStore {
    /// A configuration domain document, including every domain that
    /// persists through one (settings among them).
    Configuration,
    /// Linear history, structural envelope or payload codec.
    History,
    /// Fork-tree history.
    HistoryTree,
    /// A backup archive.
    BackupArchive,
}

impl fmt::Display for CompatibilityStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Configuration => "configuration",
            Self::History => "history",
            Self::HistoryTree => "history tree",
            Self::BackupArchive => "backup archive",
        })
    }
}

/// A load refused because the stored data was written by a newer build.
///
/// Every store already refuses forward and leaves the source untouched, but
/// each does so in its own vocabulary — a configuration recovery kind, a
/// history structural error, a payload codec error, an archive format error.
/// This is the one shape a caller can ask about without knowing which store
/// answered, so a client surface can explain a channel rejoin rather than
/// matching four unrelated error types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FutureSchemaRefusal {
    /// The store that refused.
    pub store: CompatibilityStore,
    /// Version found on disk, where the store reports one.
    pub found: Option<u32>,
    /// Highest version this build supports, where the store reports one.
    pub supported: Option<u32>,
}

impl FutureSchemaRefusal {
    /// Records a refusal whose versions are known.
    #[must_use]
    pub const fn versioned(store: CompatibilityStore, found: u32, supported: u32) -> Self {
        Self {
            store,
            found: Some(found),
            supported: Some(supported),
        }
    }

    /// Records a refusal whose versions the store does not report.
    #[must_use]
    pub const fn unversioned(store: CompatibilityStore) -> Self {
        Self {
            store,
            found: None,
            supported: None,
        }
    }
}

impl fmt::Display for FutureSchemaRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.found, self.supported) {
            (Some(found), Some(supported)) => write!(
                formatter,
                "{} data is at version {found}; this build supports {supported}",
                self.store
            ),
            _ => write!(
                formatter,
                "{} data was written by a newer build",
                self.store
            ),
        }
    }
}

/// Answers whether a load refusal was a future-schema refusal.
///
/// Implemented by each store's own refusal type. A `None` answer means the
/// refusal had some other cause and carries no claim about versions.
pub trait FutureSchemaRefused {
    /// Returns the refusal when this value represents one.
    fn future_schema_refusal(&self) -> Option<FutureSchemaRefusal>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versioned_refusal_names_both_versions() {
        let refusal = FutureSchemaRefusal::versioned(CompatibilityStore::History, 3, 2);

        assert_eq!(refusal.found, Some(3));
        assert_eq!(refusal.supported, Some(2));
        assert_eq!(
            refusal.to_string(),
            "history data is at version 3; this build supports 2"
        );
    }

    #[test]
    fn unversioned_refusal_still_states_the_cause() {
        let refusal = FutureSchemaRefusal::unversioned(CompatibilityStore::Configuration);

        assert_eq!(refusal.found, None);
        assert_eq!(
            refusal.to_string(),
            "configuration data was written by a newer build"
        );
    }
}
