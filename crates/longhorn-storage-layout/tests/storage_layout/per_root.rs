use std::path::Path;

use longhorn_storage_layout::{
    PlatformDirectoryFact, PlatformDirectoryFacts, PlatformEnvironment, RootKind, StorageIdentity,
    StorageLayoutError, StorageLayoutOverrides, StorageLayoutRequest, StorageProfile,
    TargetPlatform, native_platform_directory_fact_from, resolve_storage_layout,
    resolve_storage_root,
};

use super::support::{APP_ID, facts};

const REQUIRED_ROOTS: [RootKind; 7] = [
    RootKind::Config,
    RootKind::Data,
    RootKind::State,
    RootKind::Cache,
    RootKind::Runtime,
    RootKind::Log,
    RootKind::Backup,
];

#[test]
fn per_root_matches_complete_layout_for_every_profile_and_platform() {
    for platform in [
        TargetPlatform::MacOs,
        TargetPlatform::Windows,
        TargetPlatform::Linux,
    ] {
        let requests = [
            StorageLayoutRequest::new(StorageIdentity::new(APP_ID).unwrap(), facts(platform)),
            StorageLayoutRequest::new(
                StorageIdentity::new(APP_ID).unwrap(),
                PlatformDirectoryFacts::new(platform)
                    .with(PlatformDirectoryFact::Data, "/native/data"),
            )
            .with_profile(StorageProfile::UnifiedAppRootV1),
            StorageLayoutRequest::new(
                StorageIdentity::new(APP_ID).unwrap(),
                PlatformDirectoryFacts::new(platform)
                    .with(PlatformDirectoryFact::SharedData, "/native/shared"),
            )
            .with_profile(StorageProfile::SharedProductRootV1),
            StorageLayoutRequest::new(
                StorageIdentity::new(APP_ID).unwrap(),
                PlatformDirectoryFacts::new(platform),
            )
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root("/portable/example"),
        ];

        for request in requests {
            let layout = resolve_storage_layout(&request).unwrap();
            for kind in REQUIRED_ROOTS {
                let root = resolve_storage_root(&request, kind).unwrap();
                assert_eq!(root.as_ref(), layout.root(kind), "{platform:?} {kind:?}");
            }
            let workspace = resolve_storage_root(&request, RootKind::Workspace).unwrap();
            assert_eq!(workspace.as_ref(), layout.root(RootKind::Workspace));
        }
    }
}

#[test]
fn linux_home_resolves_config_cache_and_state_without_runtime_directory() {
    let environment = PlatformEnvironment::default().with_home_dir(Some("/home/example".into()));
    for (kind, fact, expected) in [
        (
            RootKind::Config,
            PlatformDirectoryFact::Config,
            "/home/example/.config/com.inflatablecookie.example",
        ),
        (
            RootKind::Cache,
            PlatformDirectoryFact::Cache,
            "/home/example/.cache/com.inflatablecookie.example",
        ),
        (
            RootKind::State,
            PlatformDirectoryFact::State,
            "/home/example/.local/state/com.inflatablecookie.example",
        ),
    ] {
        let facts = native_platform_directory_fact_from(TargetPlatform::Linux, fact, &environment);
        let request = StorageLayoutRequest::new(StorageIdentity::new(APP_ID).unwrap(), facts);
        let root = resolve_storage_root(&request, kind).unwrap().unwrap();
        assert_eq!(root.path(), Path::new(expected));
    }

    let runtime_facts = native_platform_directory_fact_from(
        TargetPlatform::Linux,
        PlatformDirectoryFact::Runtime,
        &environment,
    );
    let runtime_request =
        StorageLayoutRequest::new(StorageIdentity::new(APP_ID).unwrap(), runtime_facts);
    assert_eq!(
        resolve_storage_root(&runtime_request, RootKind::Runtime),
        Err(StorageLayoutError::MissingPlatformFact {
            fact: PlatformDirectoryFact::Runtime,
        })
    );
}

#[test]
fn explicit_override_resolves_without_any_platform_facts() {
    let request = StorageLayoutRequest::new(
        StorageIdentity::new(APP_ID).unwrap(),
        PlatformDirectoryFacts::new(TargetPlatform::Linux),
    )
    .with_overrides(StorageLayoutOverrides::new().with(RootKind::Config, "/chosen/config"));

    let root = resolve_storage_root(&request, RootKind::Config)
        .unwrap()
        .unwrap();
    assert_eq!(root.path(), Path::new("/chosen/config"));
}

#[test]
fn relative_xdg_cache_uses_its_default_and_does_not_change_config() {
    let environment = PlatformEnvironment::default()
        .with_home_dir(Some("/home/example".into()))
        .with_xdg_cache_home(Some("relative-cache".into()));

    let config_facts = native_platform_directory_fact_from(
        TargetPlatform::Linux,
        PlatformDirectoryFact::Config,
        &environment,
    );
    let config = resolve_storage_root(
        &StorageLayoutRequest::new(StorageIdentity::new(APP_ID).unwrap(), config_facts),
        RootKind::Config,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        config.path(),
        Path::new("/home/example/.config/com.inflatablecookie.example")
    );

    let cache_facts = native_platform_directory_fact_from(
        TargetPlatform::Linux,
        PlatformDirectoryFact::Cache,
        &environment,
    );
    let cache = resolve_storage_root(
        &StorageLayoutRequest::new(StorageIdentity::new(APP_ID).unwrap(), cache_facts),
        RootKind::Cache,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        cache.path(),
        Path::new("/home/example/.cache/com.inflatablecookie.example")
    );
}

#[test]
fn optional_roots_remain_absent_until_overridden() {
    let request = StorageLayoutRequest::new(
        StorageIdentity::new(APP_ID).unwrap(),
        PlatformDirectoryFacts::new(TargetPlatform::Linux),
    );
    assert_eq!(resolve_storage_root(&request, RootKind::Policy), Ok(None));
    assert_eq!(resolve_storage_root(&request, RootKind::Project), Ok(None));

    let request =
        request.with_overrides(StorageLayoutOverrides::new().with(RootKind::Policy, "/policy"));
    assert_eq!(
        resolve_storage_root(&request, RootKind::Policy)
            .unwrap()
            .unwrap()
            .path(),
        Path::new("/policy")
    );
}
