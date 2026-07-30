//! Tauri path mapping and injected config-operation handler assembly.
//!
//! Consumers retain filesystem selection, authorization, retention policy,
//! transition-plan custody, encryption identity, and operation execution.

mod authority;
mod commands;
mod directory;
mod error;
mod handler;
mod ports;

pub use authority::ConfigOperationsAuthority;
pub use commands::{
    ConfigOperationsCommandService, TauriConfigOperationsState, longhorn_config_backup_create,
    longhorn_config_backup_export, longhorn_config_backup_retention,
    longhorn_config_restore_adapter_execute, longhorn_config_restore_execute,
    longhorn_config_restore_inspect, longhorn_config_restore_plan, longhorn_config_restore_recover,
    longhorn_config_snapshot, longhorn_config_storage_cleanup, longhorn_config_storage_execute,
    longhorn_config_storage_inspect, longhorn_config_storage_recover,
};
pub use directory::{TauriDirectorySnapshot, platform_directory_facts};
pub use error::{ConfigOperationsHostError, ConfigOperationsHostErrorCode};
pub use handler::ConfigOperationsHandlerAssembly;
pub use ports::{
    BackupEncryptionStatusAuthority, BackupExportTargetAuthority, PortableRootAuthority,
    RestoreArchiveSelectionAuthority, RestoreUnlockAuthority, RestoreUnlockState,
};
