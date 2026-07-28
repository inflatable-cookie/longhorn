use std::{fs, time::Duration};

use longhorn_config::{
    BackupApplication, BackupArchiveInspection, BackupArchiveLimits, BackupCatalog,
    BackupExclusionReason, BackupProducer, BackupScope, BackupSourceIssue, ConfigDomain,
    ConfigStore, RestoreAction, RestoreChoices, RestoreConflictChoice, RestoreDomainCompatibility,
    RestorePlanError, RestorePrepareError, RestorePrepareOptions, StorageClass,
    encode_backup_archive, inspect_backup_archive,
};
use longhorn_core::DomainId;
use serde_json::json;

use crate::common::{Fixture, MigrationBehavior, PreferencesDomain, config_domain, document};

fn archive(store: &ConfigStore, catalog: &BackupCatalog<'_>) -> BackupArchiveInspection {
    let snapshot = super::capture(store, catalog, &BackupScope::AllRegistered).unwrap();
    let encoded = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
    inspect_backup_archive(encoded.bytes(), BackupArchiveLimits::default()).unwrap()
}

fn identities() -> (BackupApplication, BackupProducer) {
    (
        BackupApplication::new("com.example.desktop", "9.0.0").unwrap(),
        BackupProducer::new("longhorn-config", "9.0.0").unwrap(),
    )
}

fn choices(entries: impl IntoIterator<Item = (DomainId, RestoreConflictChoice)>) -> RestoreChoices {
    let mut choices = RestoreChoices::new();
    for (domain, choice) in entries {
        choices.choose(domain, choice).unwrap();
    }
    choices
}

#[path = "restore/execution.rs"]
mod execution;
#[path = "restore/inspection.rs"]
mod inspection;
#[path = "restore/planning.rs"]
mod planning;
#[path = "restore/staging.rs"]
mod staging;
