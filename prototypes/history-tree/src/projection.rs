use longhorn_core::{HistoryEntryId, HistoryRevision};
use longhorn_history::{HistoryEntryPosition, HistoryEntrySequence, HistoryLabel};

use crate::{ForkBranchId, ForkHistory};

/// Payload-free entry on the selected linear-default branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkLinearEntryProjection {
    entry_id: HistoryEntryId,
    label: HistoryLabel,
    sequence: HistoryEntrySequence,
    encoded_weight: u64,
    position: HistoryEntryPosition,
}

impl ForkLinearEntryProjection {
    /// Returns the stable entry identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns consumer-owned presentation text.
    #[must_use]
    pub const fn label(&self) -> &HistoryLabel {
        &self.label
    }

    /// Returns monotonic insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> HistoryEntrySequence {
        self.sequence
    }

    /// Returns consumer-measured payload weight, never payload bytes.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }

    /// Returns past, current, or future position on the selected path.
    #[must_use]
    pub const fn position(&self) -> HistoryEntryPosition {
        self.position
    }
}

/// Linear-default projection used when branch UI is absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkLinearProjection {
    revision: HistoryRevision,
    branch_id: ForkBranchId,
    current_entry_id: Option<HistoryEntryId>,
    entries: Vec<ForkLinearEntryProjection>,
}

impl ForkLinearProjection {
    /// Returns graph revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns selected stable branch identity.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns the current entry, or root.
    #[must_use]
    pub const fn current_entry_id(&self) -> Option<&HistoryEntryId> {
        self.current_entry_id.as_ref()
    }

    /// Returns root-to-head metadata on one path.
    #[must_use]
    pub fn entries(&self) -> &[ForkLinearEntryProjection] {
        &self.entries
    }
}

/// Optional payload-free metadata for one first-class branch reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkBranchProjection {
    branch_id: ForkBranchId,
    head_entry_id: Option<HistoryEntryId>,
    divergence_entry_id: Option<HistoryEntryId>,
    name: Option<String>,
    annotation: Option<String>,
    pinned: bool,
    current: bool,
}

impl ForkBranchProjection {
    /// Returns stable branch identity.
    #[must_use]
    pub const fn branch_id(&self) -> &ForkBranchId {
        &self.branch_id
    }

    /// Returns branch head or root.
    #[must_use]
    pub const fn head_entry_id(&self) -> Option<&HistoryEntryId> {
        self.head_entry_id.as_ref()
    }

    /// Returns divergence node relative to the current branch, or root.
    #[must_use]
    pub const fn divergence_entry_id(&self) -> Option<&HistoryEntryId> {
        self.divergence_entry_id.as_ref()
    }

    /// Returns optional name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns optional annotation.
    #[must_use]
    pub fn annotation(&self) -> Option<&str> {
        self.annotation.as_deref()
    }

    /// Returns retention pin state.
    #[must_use]
    pub const fn pinned(&self) -> bool {
        self.pinned
    }

    /// Returns whether this is the selected branch.
    #[must_use]
    pub const fn current(&self) -> bool {
        self.current
    }
}

/// Root-to-leaf path with no stable branch identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedForkPath {
    leaf_entry_id: HistoryEntryId,
    entry_ids: Vec<HistoryEntryId>,
}

impl DerivedForkPath {
    /// Returns the leaf that currently derives this path.
    #[must_use]
    pub const fn leaf_entry_id(&self) -> &HistoryEntryId {
        &self.leaf_entry_id
    }

    /// Returns root-to-leaf entry identities.
    #[must_use]
    pub fn entry_ids(&self) -> &[HistoryEntryId] {
        &self.entry_ids
    }
}

/// Optional branch-aware projection beside the linear default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkAlternateProjection {
    branches: Vec<ForkBranchProjection>,
    derived_paths: Vec<DerivedForkPath>,
}

impl ForkAlternateProjection {
    /// Returns stable first-class branch references.
    #[must_use]
    pub fn branches(&self) -> &[ForkBranchProjection] {
        &self.branches
    }

    /// Returns unstable topology-derived leaf paths for comparison.
    #[must_use]
    pub fn derived_paths(&self) -> &[DerivedForkPath] {
        &self.derived_paths
    }
}

impl<P> ForkHistory<P> {
    /// Projects only the selected branch as ordinary linear metadata.
    pub fn linear_projection(&self) -> Result<ForkLinearProjection, ForkProjectionError> {
        let branch = self
            .branches
            .get(&self.current_branch_id)
            .ok_or(ForkProjectionError::MissingCurrentBranch)?;
        let lineage = self
            .lineage::<ForkProjectionError>(branch.head_entry_id())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        let current_index = self
            .current_node_id
            .as_ref()
            .and_then(|current| lineage.iter().position(|entry_id| entry_id == current));
        let entries = lineage
            .iter()
            .enumerate()
            .map(|(index, entry_id)| {
                let node = self.nodes.get(entry_id).expect("lineage node exists");
                let position = match current_index {
                    Some(current) if index < current => HistoryEntryPosition::Past,
                    Some(current) if index == current => HistoryEntryPosition::Current,
                    _ => HistoryEntryPosition::Future,
                };
                ForkLinearEntryProjection {
                    entry_id: entry_id.clone(),
                    label: node.metadata().label().clone(),
                    sequence: node.sequence(),
                    encoded_weight: node.encoded_weight(),
                    position,
                }
            })
            .collect();
        Ok(ForkLinearProjection {
            revision: self.revision,
            branch_id: self.current_branch_id.clone(),
            current_entry_id: self.current_node_id.clone(),
            entries,
        })
    }

    /// Projects first-class references and unstable derived leaf paths.
    pub fn alternate_projection(&self) -> Result<ForkAlternateProjection, ForkProjectionError> {
        let current_branch = self
            .branches
            .get(&self.current_branch_id)
            .ok_or(ForkProjectionError::MissingCurrentBranch)?;
        let current_lineage = self
            .lineage::<ForkProjectionError>(current_branch.head_entry_id())
            .map_err(|_| ForkProjectionError::InvalidTopology)?;
        let mut branches = Vec::with_capacity(self.branches.len());
        for branch in self.branches.values() {
            let lineage = self
                .lineage::<ForkProjectionError>(branch.head_entry_id())
                .map_err(|_| ForkProjectionError::InvalidTopology)?;
            let shared = current_lineage
                .iter()
                .zip(&lineage)
                .take_while(|(left, right)| left == right)
                .count();
            branches.push(ForkBranchProjection {
                branch_id: branch.branch_id().clone(),
                head_entry_id: branch.head_entry_id().cloned(),
                divergence_entry_id: shared
                    .checked_sub(1)
                    .and_then(|index| lineage.get(index))
                    .cloned(),
                name: branch.metadata().name().map(str::to_owned),
                annotation: branch.metadata().annotation().map(str::to_owned),
                pinned: branch.metadata().pinned(),
                current: branch.branch_id() == &self.current_branch_id,
            });
        }

        let mut derived_paths = Vec::new();
        for node in self.nodes.values().filter(|node| {
            self.children
                .get(&Some(node.entry_id().clone()))
                .is_none_or(Vec::is_empty)
        }) {
            derived_paths.push(DerivedForkPath {
                leaf_entry_id: node.entry_id().clone(),
                entry_ids: self
                    .lineage::<ForkProjectionError>(Some(node.entry_id()))
                    .map_err(|_| ForkProjectionError::InvalidTopology)?,
            });
        }
        derived_paths.sort_by(|left, right| left.leaf_entry_id.cmp(&right.leaf_entry_id));
        Ok(ForkAlternateProjection {
            branches,
            derived_paths,
        })
    }
}

/// Rejected private projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkProjectionError {
    /// Current branch reference is missing.
    MissingCurrentBranch,
    /// Graph topology is invalid.
    InvalidTopology,
}
