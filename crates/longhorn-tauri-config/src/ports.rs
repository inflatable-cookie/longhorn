use std::path::PathBuf;

use longhorn_config::{BackupEncryptionState, RestoreArchiveSelection};

use crate::ConfigOperationsHostError;

/// Host-owned portable-root picker.
pub trait PortableRootAuthority {
    /// Selects one absolute root. `None` means user cancellation.
    fn select_portable_root(
        &mut self,
        caller: &str,
    ) -> Result<Option<PathBuf>, ConfigOperationsHostError>;
}

/// Host-owned backup export target picker.
pub trait BackupExportTargetAuthority {
    /// Selects one absolute export path for an already proven archive.
    ///
    /// `None` means user cancellation. The archive digest is evidence, not a
    /// renderer-provided source path.
    fn select_export_target(
        &mut self,
        caller: &str,
        archive_sha256: &str,
    ) -> Result<Option<PathBuf>, ConfigOperationsHostError>;
}

/// Redacted encryption status seam backed by a host key authority.
pub trait BackupEncryptionStatusAuthority {
    /// Returns safe availability only. Identities and passphrases stay inside
    /// the concrete provider.
    fn encryption_state(
        &mut self,
        caller: &str,
    ) -> Result<BackupEncryptionState, ConfigOperationsHostError>;
}

/// Host-owned restore archive inventory resolver and picker.
pub trait RestoreArchiveSelectionAuthority {
    /// Resolves an inventory digest or opens the host picker.
    ///
    /// `None` means cancellation or a now-absent inventory archive. The
    /// renderer never supplies a path.
    fn select_restore_archive(
        &mut self,
        caller: &str,
        selection: &RestoreArchiveSelection,
    ) -> Result<Option<PathBuf>, ConfigOperationsHostError>;
}

/// Redacted result of host-owned encrypted archive unlock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreUnlockState {
    /// Archive is plaintext and carries no authenticity claim.
    Plaintext,
    /// Encrypted envelope was authenticated and unlocked.
    Authenticated,
    /// Required noninteractive identity is unavailable.
    Locked,
    /// Host-owned interaction was cancelled.
    Cancelled,
}

/// Host-owned encryption identity and passphrase interaction seam.
pub trait RestoreUnlockAuthority {
    /// Unlocks through the concrete provider and reports only redacted state.
    ///
    /// Archive bytes and all secret material stay inside the concrete host
    /// authority.
    fn unlock_restore_archive(
        &mut self,
        caller: &str,
        archive_path: &std::path::Path,
    ) -> Result<RestoreUnlockState, ConfigOperationsHostError>;
}
