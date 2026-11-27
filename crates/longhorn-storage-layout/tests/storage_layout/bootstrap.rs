use std::fs;

use longhorn_storage_layout::{
    PlatformDirectoryFact, PlatformDirectoryFacts, StorageBootstrapOrigin,
    StorageBootstrapRecoveryKind, StorageBootstrapState, StorageIdentity, StorageLayoutDigest,
    StorageLocatorError, StorageProfile, StorageProfileLocator, StorageProfileSelection,
    TargetPlatform, encode_locator, inspect_storage_bootstrap, locator_matches,
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
                "futureField": true,
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
        (
            json!({
                "schemaVersion": 1,
                "canonicalApplicationId": "dev.nucleus.desktop",
                "profileId": "portable-v1",
                "explicitRoot": "relative"
            }),
            StorageBootstrapRecoveryKind::InvalidExplicitRoot,
        ),
        (
            json!({
                "schemaVersion": 1,
                "canonicalApplicationId": "dev.nucleus.desktop",
                "profileId": "platform-native-v1",
                "lastCommittedLayoutSha256": "bad-digest"
            }),
            StorageBootstrapRecoveryKind::InvalidLayoutDigest,
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

    fs::remove_file(paths.locator()).unwrap();
    fs::create_dir(paths.locator()).unwrap();
    let StorageBootstrapState::Recovery(recovery) =
        inspect_storage_bootstrap(&identity, &facts, None).unwrap()
    else {
        panic!("unreadable locator selected a fallback");
    };
    assert_eq!(recovery.kind(), &StorageBootstrapRecoveryKind::Unreadable);
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

#[test]
fn an_encoded_locator_writes_its_explicit_root_and_transition_as_given() {
    let selection = StorageProfileSelection::portable("/example/portable-root").unwrap();
    let locator = StorageProfileLocator::new(
        "com.example.app".to_owned(),
        selection,
        Some("example-transition".to_owned()),
        None,
    )
    .unwrap();

    let document: serde_json::Value =
        serde_json::from_slice(&encode_locator(&locator).unwrap()).unwrap();

    assert_eq!(document["canonicalApplicationId"], "com.example.app");
    assert_eq!(document["explicitRoot"], "/example/portable-root");
    assert_eq!(document["transitionId"], "example-transition");
}

#[cfg(unix)]
#[test]
fn a_non_utf8_explicit_root_is_refused_rather_than_written_lossily() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf};

    let root = PathBuf::from(OsStr::from_bytes(b"/example/portable-\xff"));
    let selection = StorageProfileSelection::portable(root.clone()).unwrap();
    let locator =
        StorageProfileLocator::new("com.example.app".to_owned(), selection, None, None).unwrap();

    let error = encode_locator(&locator).expect_err("a lossy root cannot be written");

    assert_eq!(error, StorageLocatorError::NonUtf8Root { path: root });
}

#[test]
fn locator_match_requires_exact_committed_evidence_and_a_supported_schema() {
    let temp = tempdir().unwrap();
    let facts = PlatformDirectoryFacts::new(TargetPlatform::Linux)
        .with(PlatformDirectoryFact::Config, temp.path().to_path_buf());
    let id = "com.example.app";
    let identity = StorageIdentity::new(id).unwrap();
    let paths = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    let digest = StorageLayoutDigest::new("a".repeat(64)).unwrap();
    assert!(!locator_matches(&paths, id, "commit-1", &digest).unwrap());
    fs::create_dir_all(paths.directory()).unwrap();
    let selection = StorageProfileSelection::platform_native();
    let locator = StorageProfileLocator::new(
        id.into(),
        selection.clone(),
        Some("commit-1".into()),
        Some(digest.clone()),
    )
    .unwrap();
    let bytes = encode_locator(&locator).unwrap();
    fs::write(paths.locator(), &bytes).unwrap();
    assert!(locator_matches(&paths, id, "commit-1", &digest).unwrap());
    assert!(!locator_matches(&paths, id, "commit-2", &digest).unwrap());

    let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    document["futureField"] = json!(true);
    fs::write(paths.locator(), serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(matches!(
        locator_matches(&paths, id, "commit-1", &digest),
        Err(StorageLocatorError::InvalidDocument { .. })
    ));
    document["schemaVersion"] = json!(2);
    fs::write(paths.locator(), serde_json::to_vec(&document).unwrap()).unwrap();
    assert_eq!(
        locator_matches(&paths, id, "commit-1", &digest),
        Err(StorageLocatorError::UnsupportedSchema { observed: 2 })
    );

    assert!(matches!(
        StorageProfileLocator::new("../app".into(), selection.clone(), None, None),
        Err(StorageLocatorError::InvalidApplicationId(_))
    ));
    assert_eq!(
        StorageProfileLocator::new(id.into(), selection, Some("..".into()), None),
        Err(StorageLocatorError::InvalidTransitionId)
    );
}

#[test]
fn a_legacy_locator_can_select_its_root_with_a_historical_transition_id() {
    let temp = tempdir().unwrap();
    let identity = StorageIdentity::new("com.example.app").unwrap();
    let facts = PlatformDirectoryFacts::new(TargetPlatform::Linux)
        .with(PlatformDirectoryFact::Config, temp.path().to_path_buf());
    let paths = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
    fs::create_dir_all(paths.directory()).unwrap();
    let explicit_root = temp.path().join("portable");
    fs::write(
        paths.locator(),
        serde_json::to_vec(&json!({
            "schemaVersion": 1,
            "canonicalApplicationId": "com.example.app",
            "profileId": "portable-v1",
            "explicitRoot": explicit_root,
            "transitionId": ".",
        }))
        .unwrap(),
    )
    .unwrap();
    let StorageBootstrapState::Selected(selected) =
        inspect_storage_bootstrap(&identity, &facts, None).unwrap()
    else {
        panic!("legacy locator must keep its explicit storage authority");
    };
    assert_eq!(
        selected.selection().explicit_root(),
        Some(explicit_root.as_path())
    );
    assert_eq!(selected.transition_id(), Some("."));
}
