//! Checked one-domain settings mutation over registered configuration.
//!
//! Product adapters retain schemas, policy, validation, patch, reset, and
//! activation semantics. This crate binds those adapters to one sealed apply
//! unit and one `longhorn-config` domain without claiming cross-domain
//! atomicity.

mod adapter;
mod authority;
mod binding;
mod command;
mod error;
mod execution;
mod projection;
mod registration;
mod transaction;
mod validation;

pub use adapter::{
    SettingsCommittedMutation, SettingsConfigAdapter, SettingsConfigProjection,
    SettingsConfigProjectionError,
};
pub use binding::{ConfigSettingsApplyUnit, ConfigSettingsBindingError};
pub use error::{SettingsConfigError, SettingsConfigLoadError};
pub use registration::{
    BACKUP_CREATE_CAPABILITY_ID, BACKUP_EXPORT_CAPABILITY_ID, BACKUP_INVENTORY_CAPABILITY_ID,
    BACKUP_RETENTION_CAPABILITY_ID, BACKUP_SETTINGS_PAGE_ID, BACKUP_SETTINGS_RENDERER_ID,
    CONFIG_OPERATIONS_MODULE_ID, CONFIG_OPERATIONS_SECTION_ID,
    RESTORE_ADAPTER_EXECUTION_CAPABILITY_ID, RESTORE_EXECUTION_CAPABILITY_ID,
    RESTORE_INSPECTION_CAPABILITY_ID, RESTORE_RECOVERY_CAPABILITY_ID, RESTORE_SETTINGS_PAGE_ID,
    RESTORE_SETTINGS_RENDERER_ID, STORAGE_DIAGNOSTICS_CAPABILITY_ID, STORAGE_SETTINGS_PAGE_ID,
    STORAGE_SETTINGS_RENDERER_ID, STORAGE_TRANSITION_CAPABILITY_ID,
    register_config_operations_settings,
};
pub use transaction::{ConsumerSettingsTransactionAuthority, ConsumerSettingsTransactionOutcome};
