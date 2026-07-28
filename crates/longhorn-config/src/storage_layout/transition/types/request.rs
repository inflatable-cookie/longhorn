use std::time::Duration;

use crate::{ResolvedStorageLayout, StorageBootstrapPaths, StorageProfileSelection, StorageRoots};

use super::{StorageTransitionDomain, StorageTransitionError, StorageTransitionUnknownFile};
use crate::storage_layout::transition::StorageTransitionCatalog;

/// Bounded inventory and staging limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageTransitionLimits {
    max_file_bytes: usize,
    max_total_bytes: usize,
    max_unknown_files: usize,
}

impl StorageTransitionLimits {
    /// Constructs finite non-zero limits.
    pub fn new(
        max_file_bytes: usize,
        max_total_bytes: usize,
        max_unknown_files: usize,
    ) -> Result<Self, StorageTransitionError> {
        if max_file_bytes == 0 || max_total_bytes < max_file_bytes || max_unknown_files == 0 {
            return Err(StorageTransitionError::InvalidLimits);
        }
        Ok(Self {
            max_file_bytes,
            max_total_bytes,
            max_unknown_files,
        })
    }

    pub(crate) const fn max_file_bytes(self) -> usize {
        self.max_file_bytes
    }

    pub(crate) const fn max_total_bytes(self) -> usize {
        self.max_total_bytes
    }

    pub(crate) const fn max_unknown_files(self) -> usize {
        self.max_unknown_files
    }
}

impl Default for StorageTransitionLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 64 * 1024 * 1024,
            max_total_bytes: 256 * 1024 * 1024,
            max_unknown_files: 4096,
        }
    }
}

/// Complete side-effect-free transition inspection request.
pub struct StorageTransitionRequest<'request> {
    pub(crate) source_store: &'request crate::ConfigStore,
    pub(crate) target_store: &'request crate::ConfigStore,
    pub(crate) source_layout: &'request ResolvedStorageLayout,
    pub(crate) target_layout: &'request ResolvedStorageLayout,
    pub(crate) target_selection: StorageProfileSelection,
    pub(crate) catalog: &'request StorageTransitionCatalog<'request>,
    pub(crate) bootstrap: StorageBootstrapPaths,
    pub(crate) include_logs: bool,
    pub(crate) limits: StorageTransitionLimits,
}

impl<'request> StorageTransitionRequest<'request> {
    /// Binds stores, layouts, adapter policy, and fixed bootstrap authority.
    #[must_use]
    pub fn new(
        source_store: &'request crate::ConfigStore,
        target_store: &'request crate::ConfigStore,
        source_layout: &'request ResolvedStorageLayout,
        target_layout: &'request ResolvedStorageLayout,
        target_selection: StorageProfileSelection,
        catalog: &'request StorageTransitionCatalog<'request>,
        bootstrap: StorageBootstrapPaths,
    ) -> Self {
        Self {
            source_store,
            target_store,
            source_layout,
            target_layout,
            target_selection,
            catalog,
            bootstrap,
            include_logs: false,
            limits: StorageTransitionLimits::default(),
        }
    }

    /// Includes registered logs as optional evidence.
    #[must_use]
    pub fn with_logs(mut self, include: bool) -> Self {
        self.include_logs = include;
        self
    }

    /// Replaces inventory bounds.
    #[must_use]
    pub fn with_limits(mut self, limits: StorageTransitionLimits) -> Self {
        self.limits = limits;
        self
    }
}

/// Finite execution policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTransitionExecutionOptions {
    pub(crate) transition_id: String,
    pub(crate) lock_timeout: Duration,
}

impl StorageTransitionExecutionOptions {
    /// Constructs explicit transition identity and coordinator deadline.
    pub fn new(
        transition_id: impl Into<String>,
        lock_timeout: Duration,
    ) -> Result<Self, StorageTransitionError> {
        let transition_id = transition_id.into();
        if transition_id.is_empty()
            || transition_id.len() > 128
            || !transition_id.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-')
            })
        {
            return Err(StorageTransitionError::InvalidTransitionId);
        }
        Ok(Self {
            transition_id,
            lock_timeout,
        })
    }
}

/// Explicit declarative legacy root candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyStorageCandidate {
    id: String,
    roots: StorageRoots,
}

impl LegacyStorageCandidate {
    /// Constructs a named candidate. Construction grants no authority.
    pub fn new(id: impl Into<String>, roots: StorageRoots) -> Result<Self, StorageTransitionError> {
        let id = id.into();
        if id.is_empty() || id.len() > 128 {
            return Err(StorageTransitionError::InvalidLegacyCandidate);
        }
        Ok(Self { id, roots })
    }

    /// Returns candidate id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn roots(&self) -> &StorageRoots {
        &self.roots
    }
}

/// Read-only legacy discovery result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyStorageDiscovery {
    pub(crate) candidate_id: String,
    pub(crate) domains: Vec<StorageTransitionDomain>,
    pub(crate) unknown_files: Vec<StorageTransitionUnknownFile>,
}

impl LegacyStorageDiscovery {
    /// Returns candidate id.
    #[must_use]
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    /// Returns discovered registered domains.
    #[must_use]
    pub fn domains(&self) -> &[StorageTransitionDomain] {
        &self.domains
    }

    /// Returns preserved unknown files.
    #[must_use]
    pub fn unknown_files(&self) -> &[StorageTransitionUnknownFile] {
        &self.unknown_files
    }
}
