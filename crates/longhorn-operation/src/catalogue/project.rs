//! Catalogue projection.

use std::cmp::Reverse;

use crate::OperationCatalogueProjection;

use super::OperationCatalogue;

impl OperationCatalogue {
    /// Produces active records in insertion order and terminal records newest first.
    #[must_use]
    pub fn project(&self) -> OperationCatalogueProjection {
        let mut recent: Vec<_> = self
            .operations
            .iter()
            .filter(|operation| operation.state().is_terminal())
            .cloned()
            .collect();
        recent.sort_by_key(|operation| {
            Reverse((
                operation.last_changed_catalogue_revision(),
                operation.sequence(),
            ))
        });
        OperationCatalogueProjection {
            authority: self.authority.clone(),
            catalogue_revision: self.revision,
            terminal_eviction_count: self.terminal_eviction_count,
            closed: self.closed,
            active: self
                .operations
                .iter()
                .filter(|operation| operation.is_active())
                .cloned()
                .collect(),
            recent,
        }
    }
}
