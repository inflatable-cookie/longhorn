use serde::{de, Deserialize, Deserializer, Serialize};

use super::{
    identity::deserialize_metadata, identity::deserialize_utc_timestamp, BackupApplication,
    BackupConsistencyGroup, BackupExclusion, BackupKind, BackupManifestDomain, BackupMetadata,
    BackupProducer,
};

const BACKUP_FORMAT: &str = "longhorn.config-backup";
pub(crate) const BACKUP_FORMAT_VERSION: u32 = 1;

/// Strict version-1 backup manifest model.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifest {
    #[serde(deserialize_with = "deserialize_format")]
    format: String,
    #[serde(deserialize_with = "deserialize_format_version")]
    format_version: u32,
    #[serde(deserialize_with = "deserialize_metadata")]
    archive_id: String,
    kind: BackupKind,
    #[serde(deserialize_with = "deserialize_utc_timestamp")]
    created_at: String,
    application: BackupApplication,
    producer: BackupProducer,
    consistency_groups: Vec<BackupConsistencyGroup>,
    domains: Vec<BackupManifestDomain>,
    exclusions: Vec<BackupExclusion>,
}

impl BackupManifest {
    pub(crate) fn new(
        metadata: BackupMetadata,
        consistency_groups: Vec<BackupConsistencyGroup>,
        domains: Vec<BackupManifestDomain>,
        exclusions: Vec<BackupExclusion>,
    ) -> Self {
        Self {
            format: BACKUP_FORMAT.into(),
            format_version: BACKUP_FORMAT_VERSION,
            archive_id: metadata.archive_id,
            kind: metadata.kind,
            created_at: metadata.created_at,
            application: metadata.application,
            producer: metadata.producer,
            consistency_groups,
            domains,
            exclusions,
        }
    }

    /// Returns the fixed format id.
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Returns the fixed manifest format version.
    #[must_use]
    pub const fn format_version(&self) -> u32 {
        self.format_version
    }

    /// Returns the caller-supplied archive id.
    #[must_use]
    pub fn archive_id(&self) -> &str {
        &self.archive_id
    }

    /// Returns the backup kind.
    #[must_use]
    pub const fn kind(&self) -> BackupKind {
        self.kind
    }

    /// Returns the caller-supplied UTC creation time.
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Returns application identity.
    #[must_use]
    pub fn application(&self) -> &BackupApplication {
        &self.application
    }

    /// Returns producer identity.
    #[must_use]
    pub fn producer(&self) -> &BackupProducer {
        &self.producer
    }

    /// Returns declared consistency groups.
    #[must_use]
    pub fn consistency_groups(&self) -> &[BackupConsistencyGroup] {
        &self.consistency_groups
    }

    /// Returns included domains in stable domain-id order.
    #[must_use]
    pub fn domains(&self) -> &[BackupManifestDomain] {
        &self.domains
    }

    /// Returns exclusions in stable domain-id order.
    #[must_use]
    pub fn exclusions(&self) -> &[BackupExclusion] {
        &self.exclusions
    }

    pub(crate) fn with_kind(&self, kind: BackupKind) -> Self {
        let mut manifest = self.clone();
        manifest.kind = kind;
        manifest
    }
}

fn deserialize_format<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value == BACKUP_FORMAT {
        Ok(value)
    } else {
        Err(de::Error::custom(format!(
            "unsupported backup format {value}"
        )))
    }
}

fn deserialize_format_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = u32::deserialize(deserializer)?;
    if value == BACKUP_FORMAT_VERSION {
        Ok(value)
    } else {
        Err(de::Error::custom(format!(
            "unsupported backup format version {value}"
        )))
    }
}

#[cfg(test)]
mod tests;
