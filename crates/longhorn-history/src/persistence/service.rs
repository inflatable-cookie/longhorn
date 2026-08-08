//! Registered structural and payload persistence authority.

use super::{HistoryPersistenceLimits, NoHistoryStructuralMigration};

/// Registered structural and payload persistence authority.
#[derive(Clone, Debug)]
pub struct HistoryPersistence<C, M> {
    pub(crate) codec: C,
    pub(crate) structural_migration: M,
    pub(crate) limits: HistoryPersistenceLimits,
}

impl<C, M> HistoryPersistence<C, M> {
    /// Registers a codec, structural migration hook, and byte bound.
    #[must_use]
    pub const fn new(codec: C, structural_migration: M, limits: HistoryPersistenceLimits) -> Self {
        Self {
            codec,
            structural_migration,
            limits,
        }
    }

    /// Returns the configured untrusted-byte bound.
    #[must_use]
    pub const fn limits(&self) -> HistoryPersistenceLimits {
        self.limits
    }
}

impl<C> HistoryPersistence<C, NoHistoryStructuralMigration> {
    /// Registers a codec with no older structural migration.
    #[must_use]
    pub const fn without_structural_migration(codec: C, limits: HistoryPersistenceLimits) -> Self {
        Self::new(codec, NoHistoryStructuralMigration, limits)
    }
}
