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

use std::{error::Error, fmt};

use longhorn_config::{ConfigDomain, ConfigStore, LoadOutcome, StoreError, UnavailableState};

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

/// Why `load_or_default` refused to supply a value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutLoadError {
    /// The store refused the request: an unregistered domain or a changed
    /// descriptor. That is a fault in the consumer, not a stored document.
    Store(StoreError),
    /// A restore owns the stored state, active or waiting for recovery.
    ///
    /// The restore may still replace the file, so the default must not stand in
    /// for it: a save made under the default would overwrite what the restore is
    /// about to publish or roll back. Callers resolve the restore first.
    Restore(UnavailableState),
}

impl From<StoreError> for LayoutLoadError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl fmt::Display for LayoutLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => error.fmt(formatter),
            Self::Restore(state) => write!(
                formatter,
                "layout state is owned by a restore ({state:?}); resolve it before loading"
            ),
        }
    }
}

impl Error for LayoutLoadError {}

/// Loads a document, falling back to the registered default when the stored
/// one cannot be used.
///
/// Returns the fallback reason alongside the value so a caller can log it,
/// surface it, or ignore it. Nothing is written: the unreadable source stays on
/// disk until something deliberately replaces it, so a mistake here costs one
/// session's arrangement rather than the file.
///
/// Falls back only for stored state that is unreadable (`Recovery`) and for a
/// missing optional authority (`Unavailable` with `Authority`). It does not
/// default past a restore: `RestoreActive` and `RestoreRecoveryRequired` return
/// [`LayoutLoadError::Restore`].
///
/// A `StoreError` still propagates. That is the store failing — a missing root,
/// an unregistered domain — not the document being wrong, and defaulting past
/// it would hide a real fault.
///
/// # Errors
///
/// Returns [`LayoutLoadError::Store`] for a store refusal and
/// [`LayoutLoadError::Restore`] while a restore owns the stored state.
pub fn load_or_default<D: ConfigDomain>(
    store: &ConfigStore,
    domain: &D,
) -> Result<(D::Value, LayoutFallback), LayoutLoadError> {
    Ok(match store.load(domain)? {
        LoadOutcome::Ready(loaded) => (loaded.value, LayoutFallback::None),
        LoadOutcome::Recovery(_) => (domain.default_value(), LayoutFallback::RecoveredToDefault),
        LoadOutcome::Unavailable(UnavailableState::Authority { .. }) => {
            (domain.default_value(), LayoutFallback::UnavailableToDefault)
        }
        LoadOutcome::Unavailable(
            state @ (UnavailableState::RestoreActive | UnavailableState::RestoreRecoveryRequired),
        ) => return Err(LayoutLoadError::Restore(state)),
    })
}
