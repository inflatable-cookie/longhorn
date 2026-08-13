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
        // One shape: the bundle itself is a link into a Caskroom.
        // `read_link` rather than `canonicalize`: only the link itself is the
        // signal, and canonicalize would also resolve an unrelated symlink
        // somewhere in the parent path.
        if let Ok(target) = fs::read_link(&bundle) {
            location = location.with_bundle_link_target(target.to_string_lossy());
        } else if let Some(entry) = caskroom_entry_for(&bundle, &homebrew_prefixes()) {
            // The other shape, and the one current Homebrew produces. Only
            // looked for when the bundle is *not* a link, so an install
            // already explained by the first shape costs nothing.
            location = location.with_caskroom_entry(entry.to_string_lossy());
        }
        if bundle.join("Contents/_MASReceipt/receipt").exists() {
            location = location.with_mac_app_store_receipt();
        }
        location = location.with_bundle_path(bundle.to_string_lossy());
    }

    location
}

/// Where a Caskroom might be, most likely first.
///
/// Apple silicon uses `/opt/homebrew` and Intel `/usr/local`, and either can
/// be overridden. Hard-coding one prefix would have made the fix work on the
/// machine it was written on and nowhere else.
fn homebrew_prefixes() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(prefix) = env::var("HOMEBREW_PREFIX")
        && !prefix.is_empty()
    {
        roots.push(PathBuf::from(prefix));
    }
    roots.push(PathBuf::from("/opt/homebrew"));
    roots.push(PathBuf::from("/usr/local"));
    roots
}

/// Finds a Caskroom entry that resolves to `bundle`.
///
/// A cask keeps `Caskroom/<token>/<version>/<Name>.app` as a symlink pointing
/// at wherever it put the bundle. The token is not derivable from the bundle
/// name, so the versions are walked -- but only entries whose filename already
/// matches are read, so this is a handful of `read_link` calls rather than a
/// tree scan.
fn caskroom_entry_for(bundle: &Path, prefixes: &[PathBuf]) -> Option<PathBuf> {
    let name = bundle.file_name()?;
    for prefix in prefixes {
        let caskroom = prefix.join("Caskroom");
        let Ok(tokens) = fs::read_dir(&caskroom) else {
            continue;
        };
        for token in tokens.filter_map(Result::ok) {
            let Ok(versions) = fs::read_dir(token.path()) else {
                continue;
            };
            for version in versions.filter_map(Result::ok) {
                let candidate = version.path().join(name);
                if fs::read_link(&candidate).is_ok_and(|target| target == bundle) {
                    return Some(candidate);
                }
            }
        }
    }
    None
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
        // A bundle that is itself a link into a Caskroom. This comment used to
        // claim it was "the shape Homebrew actually creates"; it is not — see
        // the test below for that one. It is kept because the shape does occur
        // and removing the branch would trade one false negative for another.
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

    /// The shape current Homebrew actually creates, and the one that was
    /// missed: the cask moves the bundle into `/Applications` and keeps the
    /// symlink in its Caskroom pointing back at it.
    ///
    /// Detected on a real machine by Card 159's packaged run, where a cask
    /// install classified as self-managed and would have been offered an
    /// in-place update.
    #[test]
    fn a_caskroom_entry_pointing_at_the_bundle_is_detected_as_a_cask() {
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Applications/Thing.app");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        fs::write(bundle.join("Contents/MacOS/thing"), b"binary").unwrap();

        let version = root.path().join("prefix/Caskroom/thing/1.0");
        fs::create_dir_all(&version).unwrap();
        std::os::unix::fs::symlink(&bundle, version.join("Thing.app")).unwrap();

        let entry = caskroom_entry_for(&bundle, &[root.path().join("prefix")]);
        assert_eq!(entry, Some(version.join("Thing.app")));

        let provenance = classify_install(
            &InstallLocation::unknown()
                .with_executable_path(bundle.join("Contents/MacOS/thing").to_string_lossy())
                .with_bundle_path(bundle.to_string_lossy())
                .with_caskroom_entry(entry.unwrap().to_string_lossy()),
        );
        assert_eq!(provenance.manager(), Some(InstallManager::HomebrewCask));
        assert!(!provenance.may_self_update());
    }

    /// The fix must not make everything look external. A bundle in the same
    /// tree with no Caskroom entry pointing at it stays self-managed.
    #[test]
    fn a_bundle_with_no_caskroom_entry_is_not_a_cask() {
        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Applications/Other.app");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        let version = root.path().join("prefix/Caskroom/thing/1.0");
        fs::create_dir_all(&version).unwrap();
        std::os::unix::fs::symlink(
            root.path().join("Applications/Thing.app"),
            version.join("Thing.app"),
        )
        .unwrap();

        assert_eq!(
            caskroom_entry_for(&bundle, &[root.path().join("prefix")]),
            None
        );
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
