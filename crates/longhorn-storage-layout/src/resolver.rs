use std::{collections::BTreeMap, path::Path};

use sha2::{Digest, Sha256};

use super::types::root_kind_id;
use super::{
    PlatformDirectoryFact, ResolvedStorageLayout, ResolvedStorageRoot, StorageLayoutError,
    StorageLayoutRequest, StorageLayoutWarning, StorageLeafProvenance, StorageProfile,
    StorageRootProvenance, TargetPlatform,
};
use crate::{RootKind, StorageLayoutDigest, StorageRoots};

/// The seven roots every complete layout resolves. A type of its own, so the
/// per-root rules cannot be asked for an optional root (policy, workspace or
/// project), and no branch of them needs an impossible case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequiredRoot {
    Config,
    Data,
    State,
    Cache,
    Runtime,
    Log,
    Backup,
}

impl RequiredRoot {
    const ALL: [Self; 7] = [
        Self::Config,
        Self::Data,
        Self::State,
        Self::Cache,
        Self::Runtime,
        Self::Log,
        Self::Backup,
    ];

    const fn kind(self) -> RootKind {
        match self {
            Self::Config => RootKind::Config,
            Self::Data => RootKind::Data,
            Self::State => RootKind::State,
            Self::Cache => RootKind::Cache,
            Self::Runtime => RootKind::Runtime,
            Self::Log => RootKind::Log,
            Self::Backup => RootKind::Backup,
        }
    }

    /// The child directory a typed profile gives this root.
    const fn child(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Data => "data",
            Self::State => "state",
            Self::Cache => "cache",
            Self::Runtime => "runtime",
            Self::Log => "logs",
            Self::Backup => "backups",
        }
    }
}

/// Resolves one complete layout without filesystem or environment access.
pub fn resolve_storage_layout(
    request: &StorageLayoutRequest,
) -> Result<ResolvedStorageLayout, StorageLayoutError> {
    validate_portable_input(request)?;

    let effective_leaf = request.identity.effective_leaf().to_owned();
    let leaf_provenance = if request.identity.stable_storage_name().is_some() {
        StorageLeafProvenance::StableStorageName
    } else {
        StorageLeafProvenance::CanonicalApplicationId
    };
    let mut roots = BTreeMap::new();

    for required in RequiredRoot::ALL {
        let resolved = resolve_required_root(request, required, &effective_leaf)?;
        roots.insert(required.kind(), resolved);
    }

    let state = roots
        .get(&RootKind::State)
        .expect("state root resolved first");
    let workspace = resolve_workspace_root(request, Some(state))?;
    roots.insert(RootKind::Workspace, workspace);
    resolve_optional_override(request, RootKind::Policy, &mut roots)?;
    resolve_optional_override(request, RootKind::Project, &mut roots)?;

    let storage_roots = build_storage_roots(&roots)?;
    let warnings = warnings(request.profile);
    let digest = layout_digest(request, &effective_leaf, leaf_provenance, &roots, &warnings);

    Ok(ResolvedStorageLayout {
        profile: request.profile,
        platform: request.facts.platform(),
        identity: request.identity.clone(),
        effective_leaf,
        leaf_provenance,
        roots,
        storage_roots,
        warnings,
        digest,
    })
}

/// Resolves one root using the same profile, fact, and override rules as a
/// complete layout.
///
/// A required root returns `Some`. Workspace is derived from the resolved
/// state root unless explicitly overridden. Policy and project roots return
/// `None` when no explicit override is configured. This projection has no
/// complete-layout digest; use [`resolve_storage_layout`] when a receipt is
/// required.
pub fn resolve_storage_root(
    request: &StorageLayoutRequest,
    kind: RootKind,
) -> Result<Option<ResolvedStorageRoot>, StorageLayoutError> {
    validate_portable_input(request)?;

    let required = match kind {
        RootKind::Policy | RootKind::Project => {
            return request
                .overrides
                .get(kind)
                .map(|path| override_root(kind, path))
                .transpose();
        }
        RootKind::Workspace => {
            let state = if request.overrides.get(RootKind::Workspace).is_some() {
                None
            } else {
                let leaf = request.identity.effective_leaf();
                Some(resolve_required_root(request, RequiredRoot::State, leaf)?)
            };
            return resolve_workspace_root(request, state.as_ref()).map(Some);
        }
        RootKind::Config => RequiredRoot::Config,
        RootKind::Data => RequiredRoot::Data,
        RootKind::State => RequiredRoot::State,
        RootKind::Cache => RequiredRoot::Cache,
        RootKind::Runtime => RequiredRoot::Runtime,
        RootKind::Log => RequiredRoot::Log,
        RootKind::Backup => RequiredRoot::Backup,
    };
    let leaf = request.identity.effective_leaf();
    resolve_required_root(request, required, leaf).map(Some)
}

fn validate_portable_input(request: &StorageLayoutRequest) -> Result<(), StorageLayoutError> {
    match (request.profile, request.portable_root.as_deref()) {
        (StorageProfile::PortableV1, None) => Err(StorageLayoutError::PortableRootRequired),
        (StorageProfile::PortableV1, Some(path)) if !path.is_absolute() => {
            Err(StorageLayoutError::InvalidPortableRoot {
                path: path.to_path_buf(),
            })
        }
        (StorageProfile::PortableV1, Some(_)) | (_, None) => Ok(()),
        (profile, Some(_)) => Err(StorageLayoutError::PortableRootForbidden { profile }),
    }
}

fn resolve_required_root(
    request: &StorageLayoutRequest,
    required: RequiredRoot,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    if let Some(path) = request.overrides.get(required.kind()) {
        return override_root(required.kind(), path);
    }

    match request.profile {
        StorageProfile::PlatformNativeV1 => native_root(request, required, leaf),
        StorageProfile::UnifiedAppRootV1 => unified_root(request, required, leaf),
        StorageProfile::SharedProductRootV1 => shared_product_root(request, required, leaf),
        StorageProfile::PortableV1 => portable_root(request, required),
    }
}

fn native_root(
    request: &StorageLayoutRequest,
    required: RequiredRoot,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let fact = native_fact(required);
    let base = required_fact(request, fact)?;
    let path = match (request.facts.platform(), required) {
        (_, RequiredRoot::Backup) => base.join(leaf).join("backups"),
        (TargetPlatform::MacOs | TargetPlatform::Windows, RequiredRoot::Config) => {
            base.join(leaf).join("config")
        }
        (TargetPlatform::MacOs | TargetPlatform::Windows, RequiredRoot::Data) => {
            base.join(leaf).join("data")
        }
        (TargetPlatform::MacOs | TargetPlatform::Windows, RequiredRoot::State) => {
            base.join(leaf).join("state")
        }
        (
            TargetPlatform::MacOs,
            RequiredRoot::Cache | RequiredRoot::Log | RequiredRoot::Runtime,
        ) => base.join(leaf),
        (TargetPlatform::Windows, RequiredRoot::Cache) => base.join(leaf).join("cache"),
        (TargetPlatform::Windows, RequiredRoot::Log) => base.join(leaf).join("logs"),
        (TargetPlatform::Windows, RequiredRoot::Runtime) => base.join(leaf),
        (TargetPlatform::Linux, RequiredRoot::Log) => base.join(leaf).join("logs"),
        (TargetPlatform::Linux, _) => base.join(leaf),
    };
    Ok(ResolvedStorageRoot::new(
        required.kind(),
        path,
        StorageRootProvenance::PlatformProfile(fact),
    ))
}

fn native_fact(required: RequiredRoot) -> PlatformDirectoryFact {
    match required {
        RequiredRoot::Config => PlatformDirectoryFact::Config,
        RequiredRoot::Data | RequiredRoot::Backup => PlatformDirectoryFact::Data,
        RequiredRoot::State => PlatformDirectoryFact::State,
        RequiredRoot::Cache => PlatformDirectoryFact::Cache,
        RequiredRoot::Runtime => PlatformDirectoryFact::Runtime,
        RequiredRoot::Log => PlatformDirectoryFact::Log,
    }
}

fn unified_root(
    request: &StorageLayoutRequest,
    required: RequiredRoot,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let base = required_fact(request, PlatformDirectoryFact::Data)?.join(leaf);
    Ok(ResolvedStorageRoot::new(
        required.kind(),
        base.join(required.child()),
        StorageRootProvenance::UnifiedProfile(PlatformDirectoryFact::Data),
    ))
}

fn shared_product_root(
    request: &StorageLayoutRequest,
    required: RequiredRoot,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let base = required_fact(request, PlatformDirectoryFact::SharedData)?.join(leaf);
    Ok(ResolvedStorageRoot::new(
        required.kind(),
        base.join(required.child()),
        StorageRootProvenance::SharedProductProfile(PlatformDirectoryFact::SharedData),
    ))
}

fn portable_root(
    request: &StorageLayoutRequest,
    required: RequiredRoot,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let base = request
        .portable_root
        .as_deref()
        .ok_or(StorageLayoutError::PortableRootRequired)?;
    Ok(ResolvedStorageRoot::new(
        required.kind(),
        base.join(required.child()),
        StorageRootProvenance::PortableProfile,
    ))
}

fn required_fact(
    request: &StorageLayoutRequest,
    fact: PlatformDirectoryFact,
) -> Result<&Path, StorageLayoutError> {
    let path = request
        .facts
        .get(fact)
        .ok_or(StorageLayoutError::MissingPlatformFact { fact })?;
    if !path.is_absolute() {
        return Err(StorageLayoutError::InvalidPlatformFact {
            fact,
            path: path.to_path_buf(),
        });
    }
    Ok(path)
}

fn override_root(kind: RootKind, path: &Path) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    if !path.is_absolute() {
        return Err(StorageLayoutError::InvalidOverride {
            kind,
            path: path.to_path_buf(),
        });
    }
    Ok(ResolvedStorageRoot::new(
        kind,
        path.to_path_buf(),
        StorageRootProvenance::ExplicitOverride,
    ))
}

fn resolve_workspace_root(
    request: &StorageLayoutRequest,
    state_root: Option<&ResolvedStorageRoot>,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let root = match request.overrides.get(RootKind::Workspace) {
        Some(path) => override_root(RootKind::Workspace, path)?,
        None => {
            let state_root = state_root.ok_or(StorageLayoutError::MissingPlatformFact {
                fact: PlatformDirectoryFact::State,
            })?;
            ResolvedStorageRoot::new(
                RootKind::Workspace,
                state_root.path().join("workspaces"),
                StorageRootProvenance::DerivedFrom(RootKind::State),
            )
        }
    };
    Ok(root)
}

fn resolve_optional_override(
    request: &StorageLayoutRequest,
    kind: RootKind,
    roots: &mut BTreeMap<RootKind, ResolvedStorageRoot>,
) -> Result<(), StorageLayoutError> {
    if let Some(path) = request.overrides.get(kind) {
        roots.insert(kind, override_root(kind, path)?);
    }
    Ok(())
}

fn build_storage_roots(
    roots: &BTreeMap<RootKind, ResolvedStorageRoot>,
) -> Result<StorageRoots, StorageLayoutError> {
    let path = |kind| {
        roots
            .get(&kind)
            .expect("required root resolved")
            .path()
            .to_path_buf()
    };
    let mut storage_roots = StorageRoots::new(
        path(RootKind::Config),
        path(RootKind::Data),
        path(RootKind::State),
        path(RootKind::Cache),
        path(RootKind::Runtime),
        path(RootKind::Log),
        path(RootKind::Backup),
    )?
    .with_workspace(path(RootKind::Workspace))?;

    if let Some(root) = roots.get(&RootKind::Policy) {
        storage_roots = storage_roots.with_policy(root.path())?;
    }
    if let Some(root) = roots.get(&RootKind::Project) {
        storage_roots = storage_roots.with_project(root.path())?;
    }
    Ok(storage_roots)
}

fn warnings(profile: StorageProfile) -> Vec<StorageLayoutWarning> {
    match profile {
        StorageProfile::PlatformNativeV1 => Vec::new(),
        StorageProfile::UnifiedAppRootV1 => vec![
            StorageLayoutWarning::UnifiedCacheLifecycle,
            StorageLayoutWarning::UnifiedRuntimeLifecycle,
            StorageLayoutWarning::UnifiedBackupClassification,
        ],
        StorageProfile::SharedProductRootV1 => vec![
            StorageLayoutWarning::SharedProductCacheLifecycle,
            StorageLayoutWarning::SharedProductLogLifecycle,
            StorageLayoutWarning::SharedProductRuntimeLifecycle,
            StorageLayoutWarning::SharedProductBackupClassification,
        ],
        StorageProfile::PortableV1 => vec![StorageLayoutWarning::PortableLifecycle],
    }
}

fn layout_digest(
    request: &StorageLayoutRequest,
    effective_leaf: &str,
    leaf_provenance: StorageLeafProvenance,
    roots: &BTreeMap<RootKind, ResolvedStorageRoot>,
    warnings: &[StorageLayoutWarning],
) -> StorageLayoutDigest {
    let mut digest = Sha256::new();
    digest_field(&mut digest, request.profile.id().as_bytes());
    digest_field(&mut digest, request.facts.platform().id().as_bytes());
    digest_field(
        &mut digest,
        request.identity.canonical_application_id().as_bytes(),
    );
    digest_field(&mut digest, effective_leaf.as_bytes());
    digest_field(&mut digest, leaf_provenance.id().as_bytes());
    for root in roots.values() {
        digest_field(&mut digest, root_kind_id(root.kind()).as_bytes());
        digest_field(&mut digest, root.path().as_os_str().as_encoded_bytes());
        digest_field(&mut digest, root.provenance().id().as_bytes());
    }
    for warning in warnings {
        digest_field(&mut digest, warning.id().as_bytes());
    }
    StorageLayoutDigest::from_output(&digest.finalize())
}

fn digest_field(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}
