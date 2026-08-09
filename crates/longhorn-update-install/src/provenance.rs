use std::{
    env, fs,
    path::{Path, PathBuf},
};

use longhorn_update::{InstallLocation, InstallManager, InstallProvenance, classify_install};

/// Observes where this application is installed.
///
/// The counterpart to [`classify_install`], which is pure. Everything that
/// touches the filesystem or the environment happens here, so the
/// classification stays testable from supplied facts and this stays a thin
/// layer of reads with no decisions in it.
///
/// Every read is best-effort. A fact that cannot be obtained is simply
/// absent, and an absent fact classifies as `Undetermined` rather than as
/// safe.
#[must_use]
pub fn observe_install(executable: &Path) -> InstallLocation {
    let mut location =
        InstallLocation::unknown().with_executable_path(executable.to_string_lossy());

    // Sandboxes announce themselves, and they are definitive, so they are
    // read first and nothing below can contradict them.
    for (variable, manager) in [
        ("FLATPAK_ID", InstallManager::Flatpak),
        ("SNAP_NAME", InstallManager::Snap),
        ("APPIMAGE", InstallManager::AppImage),
    ] {
        if let Ok(value) = env::var(variable)
            && !value.is_empty()
        {
            return location.with_sandbox(manager, value);
        }
    }

    if let Some(bundle) = macos_bundle(executable) {
        // A Homebrew cask links `/Applications/Thing.app` into its Caskroom.
        // `read_link` rather than `canonicalize`: only the link itself is the
        // signal, and canonicalize would also resolve an unrelated symlink
        // somewhere in the parent path.
        if let Ok(target) = fs::read_link(&bundle) {
            location = location.with_bundle_link_target(target.to_string_lossy());
        }
        if bundle.join("Contents/_MASReceipt/receipt").exists() {
            location = location.with_mac_app_store_receipt();
        }
        location = location.with_bundle_path(bundle.to_string_lossy());
    }

    location
}

/// Observes and classifies in one step, for the common case.
#[must_use]
pub fn detect_provenance(executable: &Path) -> InstallProvenance {
    classify_install(&observe_install(executable))
}

/// Returns the enclosing `.app` bundle for an executable inside one.
///
/// A macOS executable lives at `Thing.app/Contents/MacOS/thing`, so the
/// bundle is three levels up — and only if that directory actually ends in
/// `.app`, so a Linux path that happens to be three deep is not mistaken for
/// one.
fn macos_bundle(executable: &Path) -> Option<PathBuf> {
    let bundle = executable.parent()?.parent()?.parent()?;
    bundle
        .extension()
        .is_some_and(|extension| extension == "app")
        .then(|| bundle.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bundle_is_only_found_where_one_exists() {
        assert_eq!(
            macos_bundle(Path::new("/Applications/Thing.app/Contents/MacOS/thing")),
            Some(PathBuf::from("/Applications/Thing.app"))
        );
        // Three levels up, but not a bundle.
        assert_eq!(macos_bundle(Path::new("/usr/local/bin/thing")), None);
        assert_eq!(macos_bundle(Path::new("/thing")), None);
    }

    #[test]
    fn a_real_symlinked_bundle_is_detected_as_a_cask() {
        // Builds the shape Homebrew actually creates — a link from
        // /Applications into a Caskroom — and reads it back through the same
        // filesystem calls the real probe uses.
        let root = tempfile::tempdir().unwrap();
        let caskroom = root
            .path()
            .join("Caskroom/thing/1.0/Thing.app/Contents/MacOS");
        fs::create_dir_all(&caskroom).unwrap();
        fs::write(caskroom.join("thing"), b"binary").unwrap();

        let applications = root.path().join("Applications");
        fs::create_dir_all(&applications).unwrap();
        let linked = applications.join("Thing.app");
        std::os::unix::fs::symlink(root.path().join("Caskroom/thing/1.0/Thing.app"), &linked)
            .unwrap();

        let provenance = detect_provenance(&linked.join("Contents/MacOS/thing"));

        assert_eq!(provenance.manager(), Some(InstallManager::HomebrewCask));
        assert!(!provenance.may_self_update());
    }

    #[test]
    fn a_real_mac_app_store_receipt_is_detected() {
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Thing.app");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        fs::create_dir_all(bundle.join("Contents/_MASReceipt")).unwrap();
        fs::write(bundle.join("Contents/_MASReceipt/receipt"), b"receipt").unwrap();

        let provenance = detect_provenance(&bundle.join("Contents/MacOS/thing"));

        assert_eq!(provenance.manager(), Some(InstallManager::MacAppStore));
    }

    #[test]
    fn an_ordinary_copied_bundle_is_self_managed() {
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Thing.app");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();

        let provenance = detect_provenance(&bundle.join("Contents/MacOS/thing"));

        assert_eq!(provenance, InstallProvenance::SelfManaged);
        assert!(provenance.may_self_update());
    }
}
