use longhorn_core::HistoryEntryId;

use crate::ForkBranchId;

/// Hard limit for one private branch name.
pub const MAXIMUM_FORK_BRANCH_NAME_BYTES: usize = 256;
/// Hard limit for one private branch annotation.
pub const MAXIMUM_FORK_BRANCH_ANNOTATION_BYTES: usize = 4_096;

/// Bounded mutable metadata held outside immutable history nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranchMetadata {
    name: Option<String>,
    annotation: Option<String>,
    pinned: bool,
}

impl ForkBranchMetadata {
    /// Validates branch metadata.
    pub fn new(
        name: Option<String>,
        annotation: Option<String>,
        pinned: bool,
    ) -> Result<Self, ForkBranchMetadataError> {
        validate_optional("name", name.as_deref(), MAXIMUM_FORK_BRANCH_NAME_BYTES)?;
        validate_optional(
            "annotation",
            annotation.as_deref(),
            MAXIMUM_FORK_BRANCH_ANNOTATION_BYTES,
        )?;
        Ok(Self {
            name,
            annotation,
            pinned,
        })
    }

    /// Returns the optional branch name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the optional branch annotation.
    #[must_use]
    pub fn annotation(&self) -> Option<&str> {
        self.annotation.as_deref()
    }

    /// Returns whether retention must protect this branch.
    #[must_use]
    pub const fn pinned(&self) -> bool {
        self.pinned
    }
}

/// Invalid branch metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkBranchMetadataError {
    /// An optional value was present but empty.
    Empty {
        /// Field name.
        field: &'static str,
    },
    /// An optional value exceeded its hard limit.
    TooLong {
        /// Field name.
        field: &'static str,
        /// Maximum accepted bytes.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
}

fn validate_optional(
    field: &'static str,
    value: Option<&str>,
    maximum: usize,
) -> Result<(), ForkBranchMetadataError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_empty() {
        return Err(ForkBranchMetadataError::Empty { field });
    }
    if value.len() > maximum {
        return Err(ForkBranchMetadataError::TooLong {
            field,
            maximum,
            actual: value.len(),
        });
    }
    Ok(())
}

/// Injected identity and metadata for a newly divergent branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranchSeed {
    branch_id: ForkBranchId,
    metadata: ForkBranchMetadata,
}

impl ForkBranchSeed {
    /// Constructs a branch seed.
    #[must_use]
    pub const fn new(branch_id: ForkBranchId, metadata: ForkBranchMetadata) -> Self {
        Self {
            branch_id,
            metadata,
        }
    }

    /// Returns the injected branch identity.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns the bounded branch metadata.
    #[must_use]
    pub const fn metadata(&self) -> &ForkBranchMetadata {
        &self.metadata
    }

    pub(crate) fn into_parts(self) -> (ForkBranchId, ForkBranchMetadata) {
        (self.branch_id, self.metadata)
    }
}

/// Stable first-class reference to one branch head.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranch {
    branch_id: ForkBranchId,
    head_entry_id: Option<HistoryEntryId>,
    metadata: ForkBranchMetadata,
}

impl ForkBranch {
    pub(crate) const fn new(
        branch_id: ForkBranchId,
        head_entry_id: Option<HistoryEntryId>,
        metadata: ForkBranchMetadata,
    ) -> Self {
        Self {
            branch_id,
            head_entry_id,
            metadata,
        }
    }

    /// Returns the stable branch identity.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns the current branch head, or the root for an empty branch.
    #[must_use]
    pub const fn head_entry_id(&self) -> Option<&HistoryEntryId> {
        self.head_entry_id.as_ref()
    }

    /// Returns mutable branch-reference metadata.
    #[must_use]
    pub const fn metadata(&self) -> &ForkBranchMetadata {
        &self.metadata
    }

    pub(crate) fn set_head(&mut self, head_entry_id: Option<HistoryEntryId>) {
        self.head_entry_id = head_entry_id;
    }

    pub(crate) fn set_metadata(&mut self, metadata: ForkBranchMetadata) {
        self.metadata = metadata;
    }
}
