//! Coordinated command keymap persistence and mutation protocol.
//!
//! One versioned user-config domain stores active preset identity, monotonic
//! keymap revision, and sparse directives. Preview and commit recompile against
//! the current sealed command registry and immutable preset set.

mod domain;
mod identity;
mod migration;
mod protocol;
mod service;
mod state;

pub use domain::{
    CommandKeymapBackupPolicy, RegisteredCommandKeymapDomain, RegisteredCommandKeymapDomainError,
};
pub use identity::{CommandKeymapPatchDigest, CommandKeymapRevision};
pub use migration::{
    CommandKeymapMigration, CommandKeymapMigrationTarget, NoCommandKeymapMigration,
};
pub use protocol::{
    COMMAND_KEYMAP_PROTOCOL_VERSION, CommandCatalogueChangedEvent, CommandCatalogueSnapshot,
    CommandKeymapChangedEvent, CommandKeymapCommit, CommandKeymapCommitEvidence,
    CommandKeymapDiagnostic, CommandKeymapDurability, CommandKeymapLoadOrigin,
    CommandKeymapLoadOutcome, CommandKeymapMutationOutcome, CommandKeymapMutationReceipt,
    CommandKeymapMutationResult, CommandKeymapPatch, CommandKeymapPresetRecord,
    CommandKeymapPreview, CommandKeymapPreviewResult, CommandKeymapProtocolVersion,
    CommandKeymapRecovery, CommandKeymapRecoveryCode, CommandKeymapRejection,
    CommandKeymapRejectionCode, CommandKeymapReset, CommandKeymapSnapshot,
};
pub use service::{CommandKeymapService, CommandKeymapServiceError};
pub use state::CommandKeymapState;
