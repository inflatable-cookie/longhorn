use std::{collections::BTreeMap, path::Path};

use sha2::{Digest, Sha256};

use super::types::root_kind_id;
use super::{
    PlatformDirectoryFact, ResolvedStorageLayout, ResolvedStorageRoot, StorageLayoutError,
    StorageLayoutRequest, StorageLayoutWarning, StorageLeafProvenance, StorageProfile,
    StorageRootProvenance, TargetPlatform,
};
use crate::{RootKind, Sha256Digest, StorageRoots};

const REQUIRED_ROOTS: [RootKind; 7] = [
    RootKind::Config,
    RootKind::Data,
    RootKind::State,
    RootKind::Cache,
    RootKind::Runtime,
    RootKind::Log,
    RootKind::Backup,
];

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

    for kind in REQUIRED_ROOTS {
        let resolved = resolve_required_root(request, kind, &effective_leaf)?;
        roots.insert(kind, resolved);
    }

    resolve_workspace_root(request, &mut roots)?;
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
    kind: RootKind,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    if let Some(path) = request.overrides.get(kind) {
        return override_root(kind, path);
    }

    match request.profile {
        StorageProfile::PlatformNativeV1 => native_root(request, kind, leaf),
        StorageProfile::UnifiedAppRootV1 => unified_root(request, kind, leaf),
        StorageProfile::SharedProductRootV1 => shared_product_root(request, kind, leaf),
        StorageProfile::PortableV1 => portable_root(request, kind),
    }
}

fn native_root(
    request: &StorageLayoutRequest,
    kind: RootKind,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let fact = native_fact(kind);
    let base = required_fact(request, fact)?;
    let path = match (request.facts.platform(), kind) {
        (_, RootKind::Backup) => base.join(leaf).join("backups"),
        (TargetPlatform::MacOs | TargetPlatform::Windows, RootKind::Config) => {
            base.join(leaf).join("config")
        }
        (TargetPlatform::MacOs | TargetPlatform::Windows, RootKind::Data) => {
            base.join(leaf).join("data")
        }
        (TargetPlatform::MacOs | TargetPlatform::Windows, RootKind::State) => {
            base.join(leaf).join("state")
        }
        (TargetPlatform::MacOs, RootKind::Cache | RootKind::Log | RootKind::Runtime) => {
            base.join(leaf)
        }
        (TargetPlatform::Windows, RootKind::Cache) => base.join(leaf).join("cache"),
        (TargetPlatform::Windows, RootKind::Log) => base.join(leaf).join("logs"),
        (TargetPlatform::Windows, RootKind::Runtime) => base.join(leaf),
        (TargetPlatform::Linux, RootKind::Log) => base.join(leaf).join("logs"),
        (TargetPlatform::Linux, _) => base.join(leaf),
        (_, unsupported) => unreachable!("required native root {unsupported:?}"),
    };
    Ok(ResolvedStorageRoot::new(
        kind,
        path,
        StorageRootProvenance::PlatformProfile(fact),
    ))
}

fn native_fact(kind: RootKind) -> PlatformDirectoryFact {
    match kind {
        RootKind::Config => PlatformDirectoryFact::Config,
        RootKind::Data | RootKind::Backup => PlatformDirectoryFact::Data,
        RootKind::State => PlatformDirectoryFact::State,
        RootKind::Cache => PlatformDirectoryFact::Cache,
        RootKind::Runtime => PlatformDirectoryFact::Runtime,
        RootKind::Log => PlatformDirectoryFact::Log,
        RootKind::Policy | RootKind::Workspace | RootKind::Project => {
            unreachable!("optional roots have separate resolution")
        }
    }
}

fn unified_root(
    request: &StorageLayoutRequest,
    kind: RootKind,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let base = required_fact(request, PlatformDirectoryFact::Data)?.join(leaf);
    let child = match kind {
        RootKind::Config => "config",
        RootKind::Data => "data",
        RootKind::State => "state",
        RootKind::Cache => "cache",
        RootKind::Runtime => "runtime",
        RootKind::Log => "logs",
        RootKind::Backup => "backups",
        RootKind::Policy | RootKind::Workspace | RootKind::Project => {
            unreachable!("optional roots have separate resolution")
        }
    };
    Ok(ResolvedStorageRoot::new(
        kind,
        base.join(child),
        StorageRootProvenance::UnifiedProfile(PlatformDirectoryFact::Data),
    ))
}

fn shared_product_root(
    request: &StorageLayoutRequest,
    kind: RootKind,
    leaf: &str,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let base = required_fact(request, PlatformDirectoryFact::SharedData)?.join(leaf);
    let child = typed_child(kind);
    Ok(ResolvedStorageRoot::new(
        kind,
        base.join(child),
        StorageRootProvenance::SharedProductProfile(PlatformDirectoryFact::SharedData),
    ))
}

fn portable_root(
    request: &StorageLayoutRequest,
    kind: RootKind,
) -> Result<ResolvedStorageRoot, StorageLayoutError> {
    let base = request
        .portable_root
        .as_deref()
        .ok_or(StorageLayoutError::PortableRootRequired)?;
    let child = typed_child(kind);
    Ok(ResolvedStorageRoot::new(
        kind,
        base.join(child),
        StorageRootProvenance::PortableProfile,
    ))
}

fn typed_child(kind: RootKind) -> &'static str {
    match kind {
        RootKind::Config => "config",
        RootKind::Data => "data",
        RootKind::State => "state",
        RootKind::Cache => "cache",
        RootKind::Runtime => "runtime",
        RootKind::Log => "logs",
        RootKind::Backup => "backups",
        RootKind::Policy | RootKind::Workspace | RootKind::Project => {
            unreachable!("optional roots have separate resolution")
        }
    }
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
    roots: &mut BTreeMap<RootKind, ResolvedStorageRoot>,
) -> Result<(), StorageLayoutError> {
    let root = match request.overrides.get(RootKind::Workspace) {
        Some(path) => override_root(RootKind::Workspace, path)?,
        None => ResolvedStorageRoot::new(
            RootKind::Workspace,
            roots
                .get(&RootKind::State)
                .expect("state root resolved first")
                .path()
                .join("workspaces"),
            StorageRootProvenance::DerivedFrom(RootKind::State),
        ),
    };
    roots.insert(RootKind::Workspace, root);
    Ok(())
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
) -> Sha256Digest {
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
    Sha256Digest::new(format!("{:x}", digest.finalize())).expect("SHA-256 output is valid")
}

fn digest_field(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}
