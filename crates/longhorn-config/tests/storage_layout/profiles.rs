use std::path::PathBuf;

use longhorn_config::{
    PlatformDirectoryFact, PlatformDirectoryFacts, RootKind, StorageIdentity, StorageLayoutRequest,
    StorageLayoutWarning, StorageProfile, TargetPlatform, resolve_storage_layout,
};

use super::support::{APP_ID, assert_root, resolve_native};

#[test]
fn platform_native_matrix_uses_canonical_leaf_and_lifecycle_roots() {
    let macos = resolve_native(TargetPlatform::MacOs);
    assert_root(
        &macos,
        RootKind::Config,
        "/Users/example/Library/Application Support/com.inflatablecookie.example/config",
    );
    assert_root(
        &macos,
        RootKind::Data,
        "/Users/example/Library/Application Support/com.inflatablecookie.example/data",
    );
    assert_root(
        &macos,
        RootKind::State,
        "/Users/example/Library/Application Support/com.inflatablecookie.example/state",
    );
    assert_root(
        &macos,
        RootKind::Cache,
        "/Users/example/Library/Caches/com.inflatablecookie.example",
    );
    assert_root(
        &macos,
        RootKind::Log,
        "/Users/example/Library/Logs/com.inflatablecookie.example",
    );
    assert_root(
        &macos,
        RootKind::Runtime,
        "/private/tmp/com.inflatablecookie.example",
    );
    assert_root(
        &macos,
        RootKind::Backup,
        "/Users/example/Library/Application Support/com.inflatablecookie.example/backups",
    );

    let windows = resolve_native(TargetPlatform::Windows);
    assert_root(
        &windows,
        RootKind::Config,
        "/windows/LocalAppData/com.inflatablecookie.example/config",
    );
    assert_root(
        &windows,
        RootKind::Data,
        "/windows/LocalAppData/com.inflatablecookie.example/data",
    );
    assert_root(
        &windows,
        RootKind::State,
        "/windows/LocalAppData/com.inflatablecookie.example/state",
    );
    assert_root(
        &windows,
        RootKind::Cache,
        "/windows/LocalAppData/com.inflatablecookie.example/cache",
    );
    assert_root(
        &windows,
        RootKind::Log,
        "/windows/LocalAppData/com.inflatablecookie.example/logs",
    );
    assert_root(
        &windows,
        RootKind::Runtime,
        "/windows/Temp/com.inflatablecookie.example",
    );
    assert_root(
        &windows,
        RootKind::Backup,
        "/windows/LocalAppData/com.inflatablecookie.example/backups",
    );

    let linux = resolve_native(TargetPlatform::Linux);
    assert_root(
        &linux,
        RootKind::Config,
        "/home/example/.config/com.inflatablecookie.example",
    );
    assert_root(
        &linux,
        RootKind::Data,
        "/home/example/.local/share/com.inflatablecookie.example",
    );
    assert_root(
        &linux,
        RootKind::State,
        "/home/example/.local/state/com.inflatablecookie.example",
    );
    assert_root(
        &linux,
        RootKind::Cache,
        "/home/example/.cache/com.inflatablecookie.example",
    );
    assert_root(
        &linux,
        RootKind::Log,
        "/home/example/.local/state/com.inflatablecookie.example/logs",
    );
    assert_root(
        &linux,
        RootKind::Runtime,
        "/run/user/1000/com.inflatablecookie.example",
    );
    assert_root(
        &linux,
        RootKind::Backup,
        "/home/example/.local/share/com.inflatablecookie.example/backups",
    );
}

#[test]
fn unified_profile_matrix_needs_only_data_fact_and_reports_consequences() {
    for platform in [
        TargetPlatform::MacOs,
        TargetPlatform::Windows,
        TargetPlatform::Linux,
    ] {
        let supplied =
            PlatformDirectoryFacts::new(platform).with(PlatformDirectoryFact::Data, "/native/data");
        let layout = resolve_storage_layout(
            &StorageLayoutRequest::new(StorageIdentity::new(APP_ID).unwrap(), supplied)
                .with_profile(StorageProfile::UnifiedAppRootV1),
        )
        .unwrap();

        for (kind, child) in [
            (RootKind::Config, "config"),
            (RootKind::Data, "data"),
            (RootKind::State, "state"),
            (RootKind::Cache, "cache"),
            (RootKind::Runtime, "runtime"),
            (RootKind::Log, "logs"),
            (RootKind::Backup, "backups"),
        ] {
            assert_eq!(
                layout.root(kind).unwrap().path(),
                PathBuf::from("/native/data").join(APP_ID).join(child)
            );
        }
        assert_eq!(
            layout.warnings(),
            [
                StorageLayoutWarning::UnifiedCacheLifecycle,
                StorageLayoutWarning::UnifiedRuntimeLifecycle,
                StorageLayoutWarning::UnifiedBackupClassification,
            ]
        );
    }
}

#[test]
fn portable_profile_needs_no_platform_facts_or_leaf_overrides() {
    for platform in [
        TargetPlatform::MacOs,
        TargetPlatform::Windows,
        TargetPlatform::Linux,
    ] {
        let layout = resolve_storage_layout(
            &StorageLayoutRequest::new(
                StorageIdentity::new(APP_ID).unwrap(),
                PlatformDirectoryFacts::new(platform),
            )
            .with_profile(StorageProfile::PortableV1)
            .with_portable_root("/portable/example"),
        )
        .unwrap();

        assert_root(&layout, RootKind::Config, "/portable/example/config");
        assert_root(&layout, RootKind::Data, "/portable/example/data");
        assert_root(&layout, RootKind::State, "/portable/example/state");
        assert_root(&layout, RootKind::Cache, "/portable/example/cache");
        assert_root(&layout, RootKind::Runtime, "/portable/example/runtime");
        assert_root(&layout, RootKind::Log, "/portable/example/logs");
        assert_root(&layout, RootKind::Backup, "/portable/example/backups");
        assert_eq!(layout.warnings(), [StorageLayoutWarning::PortableLifecycle]);
    }
}
