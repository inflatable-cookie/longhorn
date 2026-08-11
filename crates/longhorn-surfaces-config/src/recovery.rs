//! Loading that survives an unreadable stored document.
//!
//! `ConfigStore::load` already refuses to guess: a document that fails to
//! decode or validate comes back as `LoadOutcome::Recovery` with the source
//! preserved, rather than as a value nobody checked. That is the right default
//! for a domain where losing state silently would be worse than stopping.
//!
//! A workspace layout is not that kind of domain. If the stored arrangement
//! cannot be read, the useful behaviour is to open on the default arrangement
//! and let the operator carry on — an application that refuses to start because
//! a panel was in the wrong place is worse than one that forgets where the
//! panel was.
//!
//! So this is opt-in, and it says so at the call site.

use longhorn_config::{ConfigDomain, ConfigStore, LoadOutcome, StoreError};

/// Why a load fell back to the registered default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutFallback {
    /// The stored document decoded and validated; nothing was discarded.
    None,
    /// The stored document could not be used and was left on disk untouched.
    ///
    /// The value in hand is the domain's registered default. The source is
    /// still recoverable through `ConfigStore::load`, which returns the
    /// preserved `RecoveryState`; this only declines to block on it.
    RecoveredToDefault,
    /// No authority could supply the document, so the default stands in.
    UnavailableToDefault,
}

impl LayoutFallback {
    /// Returns whether the stored document was discarded for this load.
    #[must_use]
    pub const fn discarded_stored_state(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Loads a document, falling back to the registered default when the stored
/// one cannot be used.
///
/// Returns the fallback reason alongside the value so a caller can log it,
/// surface it, or ignore it. Nothing is written: the unreadable source stays on
/// disk until something deliberately replaces it, so a mistake here costs one
/// session's arrangement rather than the file.
///
/// A `StoreError` still propagates. That is the store failing — a missing root,
/// an unregistered domain — not the document being wrong, and defaulting past
/// it would hide a real fault.
pub fn load_or_default<D: ConfigDomain>(
    store: &ConfigStore,
    domain: &D,
) -> Result<(D::Value, LayoutFallback), StoreError> {
    Ok(match store.load(domain)? {
        LoadOutcome::Ready(loaded) => (loaded.value, LayoutFallback::None),
        LoadOutcome::Recovery(_) => (domain.default_value(), LayoutFallback::RecoveredToDefault),
        LoadOutcome::Unavailable(_) => {
            (domain.default_value(), LayoutFallback::UnavailableToDefault)
        }
    })
}
