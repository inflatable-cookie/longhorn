mod digest;
mod domain;
mod inspect;
mod roots;
mod scanning;

use crate::RootKind;

use super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionConflict,
    StorageTransitionConflictKind, StorageTransitionDomain, StorageTransitionError,
    StorageTransitionExclusion, StorageTransitionPlan, StorageTransitionPlanError,
    StorageTransitionPreview, StorageTransitionRequest, TransitionDecision,
};

pub use inspect::{inspect_storage_transition, plan_storage_transition};

pub(crate) use digest::{confirmation_digest, evidence_digest};
pub(crate) use domain::inspect_domain;
pub(crate) use roots::{equal_root_paths, root_conflicts};
pub(crate) use scanning::scan_layout;
pub(super) use scanning::{read_evidence, scan_directory};

pub(crate) const INVENTORY_ROOTS: [RootKind; 7] = [
    RootKind::Config,
    RootKind::Data,
    RootKind::State,
    RootKind::Workspace,
    RootKind::Cache,
    RootKind::Log,
    RootKind::Runtime,
];
pub(crate) const MIGRATING_ROOTS: [RootKind; 5] = [
    RootKind::Config,
    RootKind::Data,
    RootKind::State,
    RootKind::Workspace,
    RootKind::Log,
];
