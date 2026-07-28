mod types;

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::{PlatformDirectoryFact, PlatformDirectoryFacts, StorageIdentity, TargetPlatform};
use crate::Sha256Digest;

pub use types::{
    StorageBootstrapOrigin, StorageBootstrapPathError, StorageBootstrapPaths,
    StorageBootstrapRecovery, StorageBootstrapRecoveryKind, StorageBootstrapSelection,
    StorageBootstrapState, StorageProfileLocator, StorageProfileSelection,
    StorageProfileSelectionError,
};

const LOCATOR_SCHEMA_VERSION: u32 = 1;

/// Resolves the fixed native bootstrap authority from canonical identity only.
pub fn resolve_storage_bootstrap_paths(
    identity: &StorageIdentity,
    facts: &PlatformDirectoryFacts,
) -> Result<StorageBootstrapPaths, StorageBootstrapPathError> {
    let base = facts
        .get(PlatformDirectoryFact::Config)
        .ok_or(StorageBootstrapPathError::MissingConfigFact)?;
    if !base.is_absolute() {
        return Err(StorageBootstrapPathError::InvalidConfigFact {
            path: base.to_path_buf(),
        });
    }
    let canonical = identity.canonical_application_id();
    let config_root = match facts.platform() {
        TargetPlatform::MacOs | TargetPlatform::Windows => base.join(canonical).join("config"),
        TargetPlatform::Linux => base.join(canonical),
    };
    Ok(StorageBootstrapPaths::new(config_root.join(".longhorn")))
}

/// Resolves host bypass, missing-default, locator, or recovery state.
pub fn inspect_storage_bootstrap(
    identity: &StorageIdentity,
    facts: &PlatformDirectoryFacts,
    host_bypass: Option<StorageProfileSelection>,
) -> Result<StorageBootstrapState, StorageBootstrapPathError> {
    if let Some(selection) = host_bypass {
        return Ok(StorageBootstrapState::Selected(
            StorageBootstrapSelection::new(
                selection,
                StorageBootstrapOrigin::HostBypass,
                None,
                None,
                None,
            ),
        ));
    }

    let paths = resolve_storage_bootstrap_paths(identity, facts)?;
    let bytes = match fs::read(paths.locator()) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StorageBootstrapState::Selected(
                StorageBootstrapSelection::new(
                    StorageProfileSelection::platform_native(),
                    StorageBootstrapOrigin::MissingDefault,
                    None,
                    None,
                    Some(paths),
                ),
            ));
        }
        Err(error) => {
            return Ok(recovery(
                paths,
                StorageBootstrapRecoveryKind::Unreadable,
                error.to_string(),
            ));
        }
    };

    let locator = match serde_json::from_slice::<LocatorDocument>(&bytes) {
        Ok(locator) => locator,
        Err(error) => {
            return Ok(recovery(
                paths,
                StorageBootstrapRecoveryKind::InvalidDocument,
                error.to_string(),
            ));
        }
    };
    if locator.schema_version != LOCATOR_SCHEMA_VERSION {
        return Ok(recovery(
            paths,
            StorageBootstrapRecoveryKind::UnsupportedSchema {
                observed: locator.schema_version,
            },
            "unsupported locator schema",
        ));
    }
    if locator.canonical_application_id != identity.canonical_application_id() {
        return Ok(recovery(
            paths,
            StorageBootstrapRecoveryKind::CanonicalApplicationMismatch,
            "locator canonical application id does not match host identity",
        ));
    }
    let selection = match StorageProfileSelection::from_locator(
        &locator.profile_id,
        locator.explicit_root.map(PathBuf::from),
    ) {
        Ok(selection) => selection,
        Err(error) => {
            return Ok(recovery(
                paths,
                match error {
                    StorageProfileSelectionError::UnknownProfile { .. } => {
                        StorageBootstrapRecoveryKind::UnknownProfile
                    }
                    _ => StorageBootstrapRecoveryKind::InvalidExplicitRoot,
                },
                error.to_string(),
            ));
        }
    };
    let last_committed_layout_digest = match locator.last_committed_layout_sha256 {
        Some(value) => match Sha256Digest::new(value) {
            Ok(digest) => Some(digest),
            Err(error) => {
                return Ok(recovery(
                    paths,
                    StorageBootstrapRecoveryKind::InvalidLayoutDigest,
                    error.to_string(),
                ));
            }
        },
        None => None,
    };
    let public_locator = StorageProfileLocator::new(
        locator.canonical_application_id,
        selection.clone(),
        locator.transition_id.clone(),
        last_committed_layout_digest.clone(),
    );
    Ok(StorageBootstrapState::Selected(
        StorageBootstrapSelection::new(
            selection,
            StorageBootstrapOrigin::Locator,
            locator.transition_id,
            last_committed_layout_digest,
            Some(paths),
        )
        .with_locator(public_locator),
    ))
}

pub(crate) fn encode_locator(locator: &StorageProfileLocator) -> Vec<u8> {
    serde_json::to_vec(&LocatorDocument {
        schema_version: LOCATOR_SCHEMA_VERSION,
        canonical_application_id: locator.canonical_application_id().to_owned(),
        profile_id: locator.selection().profile().id().to_owned(),
        explicit_root: locator
            .selection()
            .explicit_root()
            .map(|path| path.to_string_lossy().into_owned()),
        transition_id: locator.transition_id().map(str::to_owned),
        last_committed_layout_sha256: locator
            .last_committed_layout_digest()
            .map(|digest| digest.as_str().to_owned()),
    })
    .expect("validated storage locator is serializable")
}

pub(crate) fn locator_matches(
    paths: &StorageBootstrapPaths,
    canonical_application_id: &str,
    transition_id: &str,
    layout_digest: &Sha256Digest,
) -> Result<bool, String> {
    let bytes = match fs::read(paths.locator()) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };
    let locator =
        serde_json::from_slice::<LocatorDocument>(&bytes).map_err(|error| error.to_string())?;
    if locator.schema_version != LOCATOR_SCHEMA_VERSION {
        return Err("unsupported locator schema".into());
    }
    Ok(locator.canonical_application_id == canonical_application_id
        && locator.transition_id.as_deref() == Some(transition_id)
        && locator.last_committed_layout_sha256.as_deref() == Some(layout_digest.as_str()))
}

fn recovery(
    paths: StorageBootstrapPaths,
    kind: StorageBootstrapRecoveryKind,
    detail: impl Into<String>,
) -> StorageBootstrapState {
    StorageBootstrapState::Recovery(StorageBootstrapRecovery::new(paths, kind, detail.into()))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocatorDocument {
    schema_version: u32,
    canonical_application_id: String,
    profile_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    explicit_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transition_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_committed_layout_sha256: Option<String>,
}
