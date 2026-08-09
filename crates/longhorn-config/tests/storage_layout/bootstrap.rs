use std::fs;

use longhorn_config::{
    PlatformDirectoryFact, PlatformDirectoryFacts, StorageBootstrapOrigin,
    StorageBootstrapRecoveryKind, StorageBootstrapState, StorageIdentity, StorageProfile,
    StorageProfileSelection, TargetPlatform, inspect_storage_bootstrap,
    resolve_storage_bootstrap_paths,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn fixed_locator_uses_canonical_identity_and_missing_selects_native_default() {
    let temp = tempdir().unwrap();
    let facts = PlatformDirectoryFacts::new(TargetPlatform::MacOs).with(
        PlatformDirectoryFact::Config,
        temp.path().join("Application Support"),
    );
    let identity = StorageIdentity::new("audio.example.soundcheck")
        .unwrap()
        .with_storage_name("Soundcheck")
        .unwrap();
    let paths = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    assert!(paths.locator().ends_with(
        "Application Support/audio.example.soundcheck/config/.longhorn/storage-profile.json"
    ));
    assert!(!paths.locator().to_string_lossy().contains("Soundcheck"));

    let StorageBootstrapState::Selected(selected) =
        inspect_storage_bootstrap(&identity, &facts, None).unwrap()
    else {
        panic!("missing locator did not select default");
    };
    assert_eq!(selected.origin(), StorageBootstrapOrigin::MissingDefault);
    assert_eq!(
        selected.selection().profile(),
        StorageProfile::PlatformNativeV1
    );
}

#[test]
fn host_bypass_needs_no_bootstrap_fact_and_invalid_locators_fail_closed() {
    let identity = StorageIdentity::new("dev.nucleus.desktop").unwrap();
    let no_facts = PlatformDirectoryFacts::new(TargetPlatform::Linux);
    let portable = StorageProfileSelection::portable("/portable/nucleus").unwrap();
    let StorageBootstrapState::Selected(selected) =
        inspect_storage_bootstrap(&identity, &no_facts, Some(portable.clone())).unwrap()
    else {
        panic!("host bypass did not select");
    };
    assert_eq!(selected.origin(), StorageBootstrapOrigin::HostBypass);
    assert_eq!(selected.selection(), &portable);
    assert!(selected.paths().is_none());

    let temp = tempdir().unwrap();
    let facts = PlatformDirectoryFacts::new(TargetPlatform::Linux)
        .with(PlatformDirectoryFact::Config, temp.path().join("config"));
    let paths = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    fs::create_dir_all(paths.directory()).unwrap();
    let cases = [
        (
            json!({
                "schemaVersion": 2,
                "canonicalApplicationId": "dev.nucleus.desktop",
                "profileId": "platform-native-v1",
                "explicitRoot": null,
                "transitionId": null,
                "lastCommittedLayoutSha256": null
            }),
            StorageBootstrapRecoveryKind::UnsupportedSchema { observed: 2 },
        ),
        (
            json!({
                "schemaVersion": 1,
                "canonicalApplicationId": "other.app",
                "profileId": "platform-native-v1",
                "explicitRoot": null,
                "transitionId": null,
                "lastCommittedLayoutSha256": null
            }),
            StorageBootstrapRecoveryKind::CanonicalApplicationMismatch,
        ),
        (
            json!({
                "schemaVersion": 1,
                "canonicalApplicationId": "dev.nucleus.desktop",
                "profileId": "future-v9",
                "explicitRoot": null,
                "transitionId": null,
                "lastCommittedLayoutSha256": null
            }),
            StorageBootstrapRecoveryKind::UnknownProfile,
        ),
    ];
    for (document, expected) in cases {
        fs::write(paths.locator(), serde_json::to_vec(&document).unwrap()).unwrap();
        let StorageBootstrapState::Recovery(recovery) =
            inspect_storage_bootstrap(&identity, &facts, None).unwrap()
        else {
            panic!("invalid locator selected a fallback");
        };
        assert_eq!(recovery.kind(), &expected);
    }

    fs::write(paths.locator(), b"{not-json").unwrap();
    let StorageBootstrapState::Recovery(recovery) =
        inspect_storage_bootstrap(&identity, &facts, None).unwrap()
    else {
        panic!("corrupt locator selected a fallback");
    };
    assert_eq!(
        recovery.kind(),
        &StorageBootstrapRecoveryKind::InvalidDocument
    );
}

#[test]
fn shared_product_profile_round_trips_through_the_canonical_locator() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.inflatablecookie.loophole")
        .unwrap()
        .with_storage_name("Loophole")
        .unwrap();
    let facts = PlatformDirectoryFacts::new(TargetPlatform::Linux).with(
        PlatformDirectoryFact::Config,
        temp.path().join("xdg-config"),
    );
    let paths = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    fs::create_dir_all(paths.directory()).unwrap();
    fs::write(
        paths.locator(),
        serde_json::to_vec(&json!({
            "schemaVersion": 1,
            "canonicalApplicationId": "com.inflatablecookie.loophole",
            "profileId": "shared-product-root-v1",
            "explicitRoot": null,
            "transitionId": "loophole-storage-v1",
            "lastCommittedLayoutSha256": null
        }))
        .unwrap(),
    )
    .unwrap();

    let StorageBootstrapState::Selected(selected) =
        inspect_storage_bootstrap(&identity, &facts, None).unwrap()
    else {
        panic!("shared-product locator did not select");
    };
    assert_eq!(selected.origin(), StorageBootstrapOrigin::Locator);
    assert_eq!(
        selected.selection(),
        &StorageProfileSelection::shared_product()
    );
    assert_eq!(selected.transition_id(), Some("loophole-storage-v1"));
    assert!(
        selected
            .paths()
            .unwrap()
            .locator()
            .starts_with(temp.path().join("xdg-config/com.inflatablecookie.loophole"))
    );
}
