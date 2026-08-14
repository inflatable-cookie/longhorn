//! Exact Tauri capability and dependency boundary evidence.

use std::{fs, path::PathBuf};

/// The counts are pinned so a command added to the crate cannot quietly miss
/// the capability a host copies — the check Card 183 wished existed when a
/// fork command shipped without one.
#[test]
fn every_command_appears_in_exactly_one_permission() {
    let files = [
        ("read", file("examples/permissions/read-licence.toml")),
        ("refresh", file("examples/permissions/refresh-licence.toml")),
        (
            "activate",
            file("examples/permissions/activate-licence.toml"),
        ),
        ("seats", file("examples/permissions/seats-licence.toml")),
    ];

    for command in [
        "longhorn_licence_snapshot",
        "longhorn_licence_activate",
        "longhorn_licence_deactivate",
        "longhorn_licence_refresh",
        "longhorn_licence_release_seat",
        "longhorn_licence_rename_seat",
    ] {
        let granting: Vec<&str> = files
            .iter()
            .filter(|(_, body)| body.contains(&format!("\"{command}\"")))
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(
            granting.len(),
            1,
            "{command} is granted by {granting:?}, not by exactly one permission"
        );
    }

    let declared: usize = files
        .iter()
        .map(|(_, body)| body.matches("\"longhorn_licence_").count())
        .sum();
    assert_eq!(declared, 6, "the examples grant a command the crate lacks");
}

/// Activation carries credential material inward and writes the keychain.
/// No other grant may ride along with it.
#[test]
fn activation_is_its_own_capability_and_is_never_bundled() {
    let activate = file("examples/permissions/activate-licence.toml");
    let others = [
        file("examples/permissions/read-licence.toml"),
        file("examples/permissions/refresh-licence.toml"),
        file("examples/permissions/seats-licence.toml"),
    ];

    assert_eq!(activate.matches("\"longhorn_licence_").count(), 1);
    assert!(activate.contains("longhorn_licence_activate"));
    for other in &others {
        assert!(
            !other.contains("longhorn_licence_activate"),
            "activation must not ride along with another permission"
        );
    }

    let capability = file("examples/capabilities/activate-licence.json");
    assert!(capability.contains("allow-longhorn-licence-activate"));
    assert_eq!(
        capability.matches("allow-longhorn-licence-").count(),
        1,
        "the activation capability grants nothing else"
    );
}

/// Reading the last answer is local; a refresh reaches the backend.
#[test]
fn refreshing_is_separate_from_reading() {
    let read = file("examples/permissions/read-licence.toml");

    assert!(!read.contains("longhorn_licence_refresh"));
    assert!(read.contains("longhorn_licence_snapshot"));
}

#[test]
fn adapter_manifest_excludes_payload_and_unrelated_domains() {
    let manifest = file("Cargo.toml");
    for forbidden in [
        "longhorn-bridge",
        "longhorn-command",
        "longhorn-config",
        "longhorn-history",
        "longhorn-layout",
        "longhorn-settings",
        "longhorn-surfaces",
        "longhorn-transfer",
        "longhorn-update",
        "svelte",
        "poodle",
    ] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
    let source = format!("{}{}", file("src/authority.rs"), file("src/commands.rs"));
    assert!(!source.to_ascii_lowercase().contains("payload"));
}

/// The keychain, the browser and the activation sources are the consumer's
/// composition. This crate is the seam and nothing else.
#[test]
fn the_adapter_holds_no_composition_machinery() {
    let manifest = file("Cargo.toml");

    assert!(!manifest.contains("longhorn-credential-keyring"));
    assert!(!manifest.contains("longhorn-browser"));
    assert!(!manifest.contains("keyring"));
}

fn file(path: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}
