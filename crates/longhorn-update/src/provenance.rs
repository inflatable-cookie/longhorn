use core::fmt;

use serde::{Deserialize, Serialize};

/// Who owns the installed application.
///
/// An application a package manager installed must not replace itself. Doing
/// so leaves the manager's database describing a version that is no longer on
/// disk: `brew list --versions` reports the old one, and the next
/// `brew upgrade --cask` either reverts the update or fails on a checksum it
/// did not expect.
///
/// This is checked **before an update is offered**, not when the install is
/// attempted. Writability is the wrong signal: a Homebrew cask lands in
/// `/Applications`, which is group-writable by admin users on an ordinary
/// Mac, so a permission check passes and the desync happens quietly.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum InstallProvenance {
    /// Nothing else owns this installation; Longhorn may replace it.
    SelfManaged,
    /// A package manager or store owns it.
    ExternallyManaged {
        /// Which one.
        manager: InstallManager,
    },
    /// No detection is implemented for this platform.
    ///
    /// Treated as [`Self::SelfManaged`] by policy, because that is the
    /// behaviour that already exists and this classification must not stop
    /// ordinary installations updating. Kept distinct so a diagnostic can say
    /// "we did not check" rather than "we checked and it is fine".
    Undetermined,
}

impl InstallProvenance {
    /// Returns whether Longhorn may replace this installation itself.
    ///
    /// `Undetermined` answers yes. A false "externally managed" blocks a
    /// legitimate update, which is the worse failure of the two when nothing
    /// is known either way.
    #[must_use]
    pub const fn may_self_update(self) -> bool {
        !matches!(self, Self::ExternallyManaged { .. })
    }

    /// Returns the owning manager, when there is one.
    #[must_use]
    pub const fn manager(self) -> Option<InstallManager> {
        match self {
            Self::ExternallyManaged { manager } => Some(manager),
            Self::SelfManaged | Self::Undetermined => None,
        }
    }
}

/// A package manager or store that owns an installation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum InstallManager {
    /// Installed from the Mac App Store.
    MacAppStore,
    /// Installed as a Homebrew cask.
    HomebrewCask,
    /// Running inside a Flatpak sandbox.
    Flatpak,
    /// Running inside a Snap.
    Snap,
    /// Running from an AppImage.
    AppImage,
    /// Installed into the Nix store.
    Nix,
    /// Installed by a Linux distribution's package manager.
    LinuxDistribution,
}

impl InstallManager {
    /// Returns the name to show a user.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MacAppStore => "the Mac App Store",
            Self::HomebrewCask => "Homebrew",
            Self::Flatpak => "Flatpak",
            Self::Snap => "Snap",
            Self::AppImage => "an AppImage",
            Self::Nix => "Nix",
            Self::LinuxDistribution => "your distribution's package manager",
        }
    }

    /// Returns how the user updates, given the application's package name.
    ///
    /// `None` where the answer is not a command: the App Store updates
    /// through its own interface, and an AppImage is replaced by downloading
    /// a new one. Telling a user to run a command that does not exist is
    /// worse than telling them nothing.
    #[must_use]
    pub fn upgrade_command(self, package: &str) -> Option<String> {
        match self {
            Self::MacAppStore | Self::AppImage => None,
            Self::HomebrewCask => Some(format!("brew upgrade --cask {package}")),
            Self::Flatpak => Some(format!("flatpak update {package}")),
            Self::Snap => Some(format!("snap refresh {package}")),
            Self::Nix => Some("nix profile upgrade".to_owned()),
            Self::LinuxDistribution => None,
        }
    }
}

impl fmt::Display for InstallManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// What a host observed about where the application lives.
///
/// Facts, not conclusions, and supplied by the caller rather than read here.
/// `longhorn-update` touches no filesystem and reads no environment; the
/// probing lives in `longhorn-update-install`, which already does both.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InstallLocation {
    executable_path: Option<String>,
    bundle_path: Option<String>,
    bundle_link_target: Option<String>,
    mac_app_store_receipt: bool,
    sandbox_id: Option<(InstallManager, String)>,
}

impl InstallLocation {
    /// Records an empty observation, which classifies as `Undetermined`.
    #[must_use]
    pub fn unknown() -> Self {
        Self::default()
    }

    /// Records the running executable's path.
    #[must_use]
    pub fn with_executable_path(mut self, path: impl Into<String>) -> Self {
        self.executable_path = Some(path.into());
        self
    }

    /// Records the application bundle's path, on platforms that have one.
    #[must_use]
    pub fn with_bundle_path(mut self, path: impl Into<String>) -> Self {
        self.bundle_path = Some(path.into());
        self
    }

    /// Records what the bundle path resolves to, when it is a symlink.
    ///
    /// The Homebrew signal. A cask links `/Applications/Thing.app` to a path
    /// inside `Caskroom`, and the link is what distinguishes it from a copy
    /// the user dragged there themselves.
    #[must_use]
    pub fn with_bundle_link_target(mut self, target: impl Into<String>) -> Self {
        self.bundle_link_target = Some(target.into());
        self
    }

    /// Records that a Mac App Store receipt is present in the bundle.
    #[must_use]
    pub const fn with_mac_app_store_receipt(mut self) -> Self {
        self.mac_app_store_receipt = true;
        self
    }

    /// Records a sandbox identity read from the environment.
    #[must_use]
    pub fn with_sandbox(mut self, manager: InstallManager, id: impl Into<String>) -> Self {
        self.sandbox_id = Some((manager, id.into()));
        self
    }

    /// Returns the sandbox or package identifier, when one was observed.
    ///
    /// Useful as the package name in [`InstallManager::upgrade_command`].
    #[must_use]
    pub fn package_id(&self) -> Option<&str> {
        self.sandbox_id.as_ref().map(|(_, id)| id.as_str())
    }
}

/// Classifies an installation from what a host observed.
///
/// Ordering is deliberate: definitive signals are checked before positional
/// ones. A Flatpak's executable also sits under `/app`, and an App Store
/// application also sits in `/Applications`, so a path test that ran first
/// would answer with the weaker fact.
#[must_use]
pub fn classify_install(location: &InstallLocation) -> InstallProvenance {
    let managed = |manager| InstallProvenance::ExternallyManaged { manager };

    // Definitive: the environment or the bundle says so outright.
    if location.mac_app_store_receipt {
        return managed(InstallManager::MacAppStore);
    }
    if let Some((manager, _)) = location.sandbox_id {
        return managed(manager);
    }

    // Strong: a symlink into a Caskroom is what a cask looks like.
    if let Some(target) = &location.bundle_link_target
        && target.contains("/Caskroom/")
    {
        return managed(InstallManager::HomebrewCask);
    }

    let Some(executable) = &location.executable_path else {
        return InstallProvenance::Undetermined;
    };

    if executable.starts_with("/nix/store/") {
        return managed(InstallManager::Nix);
    }
    // A distribution owns /usr. /usr/local is the documented exception:
    // it is where a machine's owner puts things, which is why package
    // managers are told not to write there.
    if executable.starts_with("/usr/") && !executable.starts_with("/usr/local/") {
        return managed(InstallManager::LinuxDistribution);
    }

    // A recognised layout with no external owner. Anything unrecognised —
    // every Windows layout today — stays undetermined rather than claiming
    // to be safe.
    if location.bundle_path.is_some()
        || executable.starts_with("/Applications/")
        || executable.starts_with("/usr/local/")
        || executable.starts_with("/opt/")
        || executable.contains("/.local/")
    {
        return InstallProvenance::SelfManaged;
    }

    InstallProvenance::Undetermined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_homebrew_cask_is_externally_managed_even_though_it_is_writable() {
        // The whole point. `/Applications` is group-writable by admin users,
        // so a permission check passes and the app happily replaces itself
        // while brew's database goes stale.
        let location = InstallLocation::unknown()
            .with_executable_path("/Applications/Soundcheck.app/Contents/MacOS/soundcheck")
            .with_bundle_path("/Applications/Soundcheck.app")
            .with_bundle_link_target("/opt/homebrew/Caskroom/soundcheck/0.1.0/Soundcheck.app");

        let provenance = classify_install(&location);

        assert_eq!(provenance.manager(), Some(InstallManager::HomebrewCask));
        assert!(!provenance.may_self_update());
        assert_eq!(
            InstallManager::HomebrewCask.upgrade_command("soundcheck"),
            Some("brew upgrade --cask soundcheck".to_owned())
        );
    }

    #[test]
    fn the_same_bundle_dragged_into_applications_is_self_managed() {
        // No symlink, no receipt. The user put it there and owns it.
        let location = InstallLocation::unknown()
            .with_executable_path("/Applications/Soundcheck.app/Contents/MacOS/soundcheck")
            .with_bundle_path("/Applications/Soundcheck.app");

        assert_eq!(classify_install(&location), InstallProvenance::SelfManaged);
    }

    #[test]
    fn a_mac_app_store_receipt_outranks_everything_else() {
        let location = InstallLocation::unknown()
            .with_executable_path("/Applications/Soundcheck.app/Contents/MacOS/soundcheck")
            .with_bundle_path("/Applications/Soundcheck.app")
            .with_mac_app_store_receipt();

        let provenance = classify_install(&location);

        assert_eq!(provenance.manager(), Some(InstallManager::MacAppStore));
        // No command: the App Store updates through its own interface, and
        // inventing one would be worse than saying nothing.
        assert_eq!(InstallManager::MacAppStore.upgrade_command("x"), None);
    }

    #[test]
    fn sandbox_identity_is_definitive_and_beats_the_path_it_implies() {
        // A Flatpak's executable also lives under a path a positional test
        // could misread, so the environment signal is checked first.
        for (manager, id, command) in [
            (
                InstallManager::Flatpak,
                "com.example.Soundcheck",
                Some("flatpak update com.example.Soundcheck".to_owned()),
            ),
            (
                InstallManager::Snap,
                "soundcheck",
                Some("snap refresh soundcheck".to_owned()),
            ),
            (InstallManager::AppImage, "Soundcheck.AppImage", None),
        ] {
            let location = InstallLocation::unknown()
                .with_executable_path("/usr/bin/soundcheck")
                .with_sandbox(manager, id);

            let provenance = classify_install(&location);

            assert_eq!(provenance.manager(), Some(manager), "{manager:?}");
            assert_eq!(manager.upgrade_command(id), command, "{manager:?}");
        }
    }

    #[test]
    fn a_distribution_owns_usr_but_not_usr_local() {
        let distribution = InstallLocation::unknown().with_executable_path("/usr/bin/soundcheck");
        assert_eq!(
            classify_install(&distribution).manager(),
            Some(InstallManager::LinuxDistribution)
        );

        // /usr/local is where the machine's owner puts things, which is
        // exactly why package managers are told to stay out of it.
        let local = InstallLocation::unknown().with_executable_path("/usr/local/bin/soundcheck");
        assert_eq!(classify_install(&local), InstallProvenance::SelfManaged);
    }

    #[test]
    fn the_nix_store_is_externally_managed() {
        let location = InstallLocation::unknown()
            .with_executable_path("/nix/store/abc123-soundcheck-0.1.0/bin/soundcheck");

        assert_eq!(
            classify_install(&location).manager(),
            Some(InstallManager::Nix)
        );
    }

    #[test]
    fn an_unrecognised_layout_is_undetermined_and_still_updates() {
        // Every Windows layout today. Undetermined must not stop an ordinary
        // installation updating, because that would be a regression against
        // the behaviour that exists now.
        let windows = InstallLocation::unknown()
            .with_executable_path(r"C:\Program Files\Soundcheck\soundcheck.exe");

        let provenance = classify_install(&windows);

        assert_eq!(provenance, InstallProvenance::Undetermined);
        assert!(provenance.may_self_update());
        assert_eq!(provenance.manager(), None);
    }

    #[test]
    fn nothing_observed_is_undetermined_rather_than_safe() {
        assert_eq!(
            classify_install(&InstallLocation::unknown()),
            InstallProvenance::Undetermined
        );
    }
}
