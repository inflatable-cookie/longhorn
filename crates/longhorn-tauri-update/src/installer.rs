use core::fmt;
use std::error::Error;

/// Performs the download, verification, and replacement.
///
/// Injected rather than bound to `tauri-plugin-updater` for two reasons.
///
/// The interlock is the part only Longhorn can write, and it is fully
/// testable behind this port; binding the plugin here would make the
/// valuable half unverifiable without a packaged application.
///
/// And the concrete plugin-backed installer cannot be exercised headlessly
/// at all — macOS bundle replacement and relaunch need a real installed
/// application, and tauri#11392 puts the relaunch path specifically in
/// doubt. It therefore lands with the packaged proof (Card 159), where it
/// can be proved rather than assumed.
///
/// An implementation **must not** verify signatures itself. Verification
/// belongs to the plugin.
pub trait UpdateInstaller {
    /// Downloads, verifies, and replaces the installed application.
    ///
    /// Returns once the replacement is on disk. Relaunching is separate:
    /// macOS `install` deliberately does not relaunch, which is what lets
    /// the caller order quiesce, install, tear down, and relaunch itself.
    fn install(&self) -> Result<(), InstallError>;

    /// Relaunches the application.
    fn relaunch(&self) -> Result<(), InstallError>;
}

/// Installation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstallError {
    /// The installed application could not be written.
    ///
    /// Homebrew casks and administrator-installed copies land here. The
    /// remedy is a manual download, not a retry, and the caller is expected
    /// to say so rather than surface an error.
    NotWritable {
        /// What could not be written.
        detail: String,
    },
    /// The download or replacement failed.
    Failed {
        /// What went wrong.
        detail: String,
    },
    /// The replacement succeeded but the relaunch did not.
    ///
    /// Recorded distinctly because the update *did* land: telling the user
    /// to reopen the application is correct, and telling them the update
    /// failed is not.
    RelaunchFailed {
        /// What went wrong.
        detail: String,
    },
}

impl InstallError {
    /// Returns whether the update reached disk despite the error.
    #[must_use]
    pub const fn update_landed(&self) -> bool {
        matches!(self, Self::RelaunchFailed { .. })
    }
}

impl fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotWritable { detail } => {
                write!(formatter, "installation is not writable: {detail}")
            }
            Self::Failed { detail } => write!(formatter, "update install failed: {detail}"),
            Self::RelaunchFailed { detail } => write!(
                formatter,
                "update installed but the application did not relaunch: {detail}"
            ),
        }
    }
}

impl Error for InstallError {}
