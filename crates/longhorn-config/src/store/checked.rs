use std::{error::Error, fmt};

use longhorn_core::SchemaVersion;

use super::{LoadDiagnostic, LoadedOrigin, MutationError, MutationReceipt, SourceDocument};

/// Fresh typed state exposed to one checked patch while coordination is held.
///
/// The context cannot publish independently. The store validates and encodes
/// the final value only after the check returns successfully.
pub struct CheckedMutationContext<'value, T> {
    pub(crate) value: &'value mut T,
    pub(crate) schema_version: SchemaVersion,
    pub(crate) origin: LoadedOrigin,
    pub(crate) diagnostics: &'value [LoadDiagnostic],
    pub(crate) source: Option<&'value SourceDocument>,
}

impl<T> CheckedMutationContext<'_, T> {
    /// Returns the fresh decoded value.
    #[must_use]
    pub const fn value(&self) -> &T {
        self.value
    }

    /// Returns the fresh decoded value for a checked patch.
    #[must_use]
    pub const fn value_mut(&mut self) -> &mut T {
        self.value
    }

    /// Returns the loaded schema version.
    #[must_use]
    pub const fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns whether the fresh value came from default, file, or migration.
    #[must_use]
    pub const fn origin(&self) -> LoadedOrigin {
        self.origin
    }

    /// Returns non-fatal diagnostics from the coordinated load.
    #[must_use]
    pub const fn diagnostics(&self) -> &[LoadDiagnostic] {
        self.diagnostics
    }

    /// Returns exact source bytes when a file was read.
    #[must_use]
    pub const fn source(&self) -> Option<&SourceDocument> {
        self.source
    }
}

/// Accepted checked patch plus optional publication evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedMutationOutcome<R, T> {
    checked: R,
    value: T,
    publication: Option<MutationReceipt>,
}

impl<R, T> CheckedMutationOutcome<R, T> {
    pub(crate) const fn new(checked: R, value: T, publication: Option<MutationReceipt>) -> Self {
        Self {
            checked,
            value,
            publication,
        }
    }

    /// Returns the caller's checked result.
    #[must_use]
    pub const fn checked(&self) -> &R {
        &self.checked
    }

    /// Consumes the outcome and returns the caller's checked result.
    #[must_use]
    pub fn into_checked(self) -> R {
        self.checked
    }

    /// Returns the validated final value accepted by the mutation.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Consumes the outcome into checked output, final value, and publication.
    #[must_use]
    pub fn into_parts(self) -> (R, T, Option<MutationReceipt>) {
        (self.checked, self.value, self.publication)
    }

    /// Returns publication evidence, or `None` when bytes were unchanged.
    #[must_use]
    pub const fn publication(&self) -> Option<&MutationReceipt> {
        self.publication.as_ref()
    }

    /// Returns whether the accepted patch changed authoritative bytes.
    #[must_use]
    pub const fn changed(&self) -> bool {
        self.publication.is_some()
    }
}

/// Checked-patch veto or underlying coordinated mutation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedMutationError<E> {
    /// The caller rejected fresh state before publication.
    Check(E),
    /// Registration, recovery, validation, encoding, or publication failed.
    Mutation(MutationError),
}

impl<E> fmt::Display for CheckedMutationError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Check(error) => error.fmt(formatter),
            Self::Mutation(error) => error.fmt(formatter),
        }
    }
}

impl<E> Error for CheckedMutationError<E> where E: Error + 'static {}
