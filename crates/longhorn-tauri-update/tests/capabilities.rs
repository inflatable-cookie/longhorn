//! Exact Tauri capability and dependency boundary evidence.

use std::{fs, path::PathBuf};

/// The counts are pinned so a command added to the crate cannot quietly miss
/// the capability a host copies. Card 183 hit exactly that in the fork-history
/// adapter: the command shipped, the example did not list it, and a host
/// copying the example was denied at runtime by a message naming a permission
/// rather than a missing command.
#[test]
fn every_command_appears_in_exactly_one_permission() {
    let files = [
        ("read", file("examples/permissions/read-update.toml")),
        ("check", file("examples/permissions/check-update.toml")),
        ("mutate", file("examples/permissions/mutate-update.toml")),
        ("install", file("examples/permissions/install-update.toml")),
    ];

    for command in [
        "longhorn_update_snapshot",
        "longhorn_update_check",
        "longhorn_update_select_channel",
        "longhorn_update_defer",
        "longhorn_update_install",
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
        .map(|(_, body)| body.matches("\"longhorn_update_").count())
        .sum();
    assert_eq!(declared, 5, "the examples grant a command the crate lacks");
}

/// Card 190 step 2. Authorizing an install is not covered by permission to
/// look for one: the first reads, the second replaces the running application.
#[test]
fn installing_is_its_own_capability_and_is_never_bundled_with_checking() {
    let install = file("examples/permissions/install-update.toml");
    let check = file("examples/permissions/check-update.toml");
    let read = file("examples/permissions/read-update.toml");
    let mutate = file("examples/permissions/mutate-update.toml");

    assert_eq!(install.matches("\"longhorn_update_").count(), 1);
    assert!(install.contains("longhorn_update_install"));
    for other in [&check, &read, &mutate] {
        assert!(
            !other.contains("longhorn_update_install"),
            "install must not ride along with another permission"
        );
    }

    let capability = file("examples/capabilities/install-update.json");
    assert!(capability.contains("allow-longhorn-update-install"));
    assert_eq!(
        capability.matches("allow-longhorn-update-").count(),
        1,
        "the install capability grants nothing else"
    );
}

/// Reading the last answer is local; running a check reaches the network.
#[test]
fn checking_is_separate_from_reading() {
    let read = file("examples/permissions/read-update.toml");

    assert!(!read.contains("longhorn_update_check"));
    assert!(read.contains("longhorn_update_snapshot"));
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
        "svelte",
        "poodle",
    ] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
    let source = format!("{}{}", file("src/authority.rs"), file("src/commands.rs"));
    assert!(!source.to_ascii_lowercase().contains("payload"));
}

/// The installer, the signing key and the transfer are the consumer's. This
/// crate is the seam and nothing else -- the distinction the absorbed crate of
/// the same name did not draw.
#[test]
fn the_adapter_holds_no_install_machinery() {
    let manifest = file("Cargo.toml");

    assert!(!manifest.contains("longhorn-update-install"));
    assert!(!manifest.contains("minisign"));
}

fn file(path: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}
