use std::path::{Path, PathBuf};

use longhorn_config::{
    PlatformDirectoryFact, PlatformDirectoryFacts, RootKind, StorageIdentity, StorageLayoutError,
    StorageLayoutOverrides, StorageLayoutRequest, StorageLeafProvenance, StorageProfile,
    StorageRootProvenance, TargetPlatform, resolve_storage_layout,
};

use super::support::{APP_ID, assert_root, facts, resolve_native};

#[test]
fn stable_storage_name_replaces_every_profile_leaf_with_provenance() {
    let identity = StorageIdentity::new("audio.infiniteloop.soundcheck")
        .unwrap()
        .with_storage_name("Soundcheck")
        .unwrap();
    let native = resolve_storage_layout(&StorageLayoutRequest::new(
        identity.clone(),
        facts(TargetPlatform::MacOs),
    ))
    .unwrap();
    assert_eq!(native.effective_leaf(), "Soundcheck");
    assert_eq!(
        native.leaf_provenance(),
        StorageLeafProvenance::StableStorageName
    );
    assert_root(
        &native,
        RootKind::Data,
        "/Users/example/Library/Application Support/Soundcheck/data",
    );
    assert_root(
        &native,
        RootKind::Cache,
        "/Users/example/Library/Caches/Soundcheck",
    );

    let unified = resolve_storage_layout(
        &StorageLayoutRequest::new(identity, facts(TargetPlatform::MacOs))
            .with_profile(StorageProfile::UnifiedAppRootV1),
    )
    .unwrap();
    assert_root(
        &unified,
        RootKind::Data,
        "/Users/example/Library/Application Support/Soundcheck/data",
    );
}

#[test]
fn overrides_are_exact_visible_and_can_replace_missing_profile_facts() {
    let mut overrides = StorageLayoutOverrides::new();
    for kind in [
        RootKind::Config,
        RootKind::Data,
        RootKind::State,
        RootKind::Cache,
        RootKind::Runtime,
        RootKind::Log,
        RootKind::Backup,
        RootKind::Workspace,
        RootKind::Policy,
        RootKind::Project,
    ] {
        overrides = overrides.with(kind, format!("/override/{kind:?}").to_lowercase());
    }
    let layout = resolve_storage_layout(
        &StorageLayoutRequest::new(
            StorageIdentity::new(APP_ID).unwrap(),
            PlatformDirectoryFacts::new(TargetPlatform::Linux),
        )
        .with_overrides(overrides),
    )
    .unwrap();

    for root in layout.diagnostic().roots() {
        assert_eq!(root.provenance(), StorageRootProvenance::ExplicitOverride);
        assert!(root.path().starts_with("/override"));
    }
}

#[test]
fn missing_relative_and_invalid_profile_inputs_fail_typed() {
    let identity = StorageIdentity::new(APP_ID).unwrap();
    assert_eq!(
        resolve_storage_layout(&StorageLayoutRequest::new(
            identity.clone(),
            PlatformDirectoryFacts::new(TargetPlatform::Linux),
        )),
        Err(StorageLayoutError::MissingPlatformFact {
            fact: PlatformDirectoryFact::Config,
        })
    );

    let relative = PlatformDirectoryFacts::new(TargetPlatform::Linux)
        .with(PlatformDirectoryFact::Config, "relative");
    assert_eq!(
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), relative)),
        Err(StorageLayoutError::InvalidPlatformFact {
            fact: PlatformDirectoryFact::Config,
            path: PathBuf::from("relative"),
        })
    );

    let empty =
        PlatformDirectoryFacts::new(TargetPlatform::Linux).with(PlatformDirectoryFact::Config, "");
    assert_eq!(
        resolve_storage_layout(&StorageLayoutRequest::new(identity.clone(), empty)),
        Err(StorageLayoutError::InvalidPlatformFact {
            fact: PlatformDirectoryFact::Config,
            path: PathBuf::new(),
        })
    );

    assert_eq!(
        resolve_storage_layout(
            &StorageLayoutRequest::new(
                identity.clone(),
                PlatformDirectoryFacts::new(TargetPlatform::Linux),
            )
            .with_profile(StorageProfile::PortableV1),
        ),
        Err(StorageLayoutError::PortableRootRequired)
    );
    assert!(StorageProfile::from_id("platform-native-v2").is_err());
}

#[test]
fn built_in_profile_ids_are_exact_and_round_trip() {
    for (profile, id) in [
        (StorageProfile::PlatformNativeV1, "platform-native-v1"),
        (StorageProfile::UnifiedAppRootV1, "unified-app-root-v1"),
        (
            StorageProfile::SharedProductRootV1,
            "shared-product-root-v1",
        ),
        (StorageProfile::PortableV1, "portable-v1"),
    ] {
        assert_eq!(profile.id(), id);
        assert_eq!(StorageProfile::from_id(id), Ok(profile));
    }
}

#[test]
fn digest_and_diagnostic_bind_profile_identity_roots_and_provenance() {
    let first = resolve_native(TargetPlatform::MacOs);
    let second = resolve_native(TargetPlatform::MacOs);
    assert_eq!(first.digest(), second.digest());
    assert_eq!(first.diagnostic().digest(), first.digest());
    assert_eq!(
        first.diagnostic().leaf_provenance(),
        StorageLeafProvenance::CanonicalApplicationId
    );

    let overridden = resolve_storage_layout(
        &StorageLayoutRequest::new(
            StorageIdentity::new(APP_ID).unwrap(),
            facts(TargetPlatform::MacOs),
        )
        .with_overrides(StorageLayoutOverrides::new().with(RootKind::Cache, "/override/cache")),
    )
    .unwrap();
    assert_ne!(first.digest(), overridden.digest());
}

#[test]
fn workspace_and_database_conventions_follow_state_and_lifecycle() {
    let layout = resolve_native(TargetPlatform::Linux);
    assert_root(
        &layout,
        RootKind::Workspace,
        "/home/example/.local/state/com.inflatablecookie.example/workspaces",
    );
    assert_eq!(
        layout.durable_database_dir(),
        Path::new("/home/example/.local/share/com.inflatablecookie.example/databases")
    );
    assert_eq!(
        layout.state_database_dir(),
        Path::new("/home/example/.local/state/com.inflatablecookie.example/databases")
    );
    assert_eq!(
        layout.cache_database_dir(),
        Path::new("/home/example/.cache/com.inflatablecookie.example/databases")
    );
}

#[test]
fn donor_identities_map_without_display_name_inference() {
    let cases = [
        (
            "audio.infiniteloop.soundcheck",
            Some("Soundcheck"),
            "Soundcheck",
        ),
        (
            "audio.infiniteloop.loophole.aura",
            Some("Loophole"),
            "Loophole",
        ),
        (
            "com.acowtancy.bovine-accelerator",
            None,
            "com.acowtancy.bovine-accelerator",
        ),
        ("dev.nucleus.desktop", Some("Nucleus"), "Nucleus"),
    ];

    for (application_id, storage_name, expected_leaf) in cases {
        let mut identity = StorageIdentity::new(application_id).unwrap();
        if let Some(storage_name) = storage_name {
            identity = identity.with_storage_name(storage_name).unwrap();
        }
        let layout = resolve_storage_layout(&StorageLayoutRequest::new(
            identity,
            facts(TargetPlatform::MacOs),
        ))
        .unwrap();
        assert_eq!(layout.effective_leaf(), expected_leaf);
    }
}
