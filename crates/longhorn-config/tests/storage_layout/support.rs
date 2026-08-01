use std::path::Path;

use longhorn_config::{
    PlatformDirectoryFact, PlatformDirectoryFacts, ResolvedStorageLayout, RootKind,
    StorageIdentity, StorageLayoutRequest, TargetPlatform, resolve_storage_layout,
};

pub(crate) const APP_ID: &str = "com.inflatablecookie.example";

pub(crate) fn facts(platform: TargetPlatform) -> PlatformDirectoryFacts {
    match platform {
        TargetPlatform::MacOs => PlatformDirectoryFacts::complete(
            platform,
            "/Users/example/Library/Application Support",
            "/Users/example/Library/Application Support",
            "/Users/example/Library/Application Support",
            "/Users/example/Library/Caches",
            "/Users/example/Library/Logs",
            "/private/tmp",
        )
        .with(
            PlatformDirectoryFact::SharedData,
            "/Users/example/Library/Application Support",
        ),
        TargetPlatform::Windows => PlatformDirectoryFacts::complete(
            platform,
            "/windows/LocalAppData",
            "/windows/LocalAppData",
            "/windows/LocalAppData",
            "/windows/LocalAppData",
            "/windows/LocalAppData",
            "/windows/Temp",
        )
        .with(PlatformDirectoryFact::SharedData, "/windows/RoamingAppData"),
        TargetPlatform::Linux => PlatformDirectoryFacts::complete(
            platform,
            "/home/example/.config",
            "/home/example/.local/share",
            "/home/example/.local/state",
            "/home/example/.cache",
            "/home/example/.local/state",
            "/run/user/1000",
        )
        .with(
            PlatformDirectoryFact::SharedData,
            "/home/example/.local/share",
        ),
    }
}

pub(crate) fn resolve_native(platform: TargetPlatform) -> ResolvedStorageLayout {
    resolve_storage_layout(&StorageLayoutRequest::new(
        StorageIdentity::new(APP_ID).unwrap(),
        facts(platform),
    ))
    .unwrap()
}

pub(crate) fn assert_root(layout: &ResolvedStorageLayout, kind: RootKind, expected: &str) {
    assert_eq!(layout.root(kind).unwrap().path(), Path::new(expected));
}
