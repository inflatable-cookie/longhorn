use std::fmt;

/// The platform facilities a host supplies that Longhorn will not assume.
///
/// A webview is a platform as well as a renderer. A Svelte surface reaches for
/// `crypto.randomUUID`, `toLocaleString` and `toLocaleLowerCase` and gets all
/// three for nothing; a native surface has no equivalent without a dependency
/// and a locale, neither of which belongs in a domain or projection crate.
///
/// Bundled rather than injected one at a time so an application author has a
/// single thing to satisfy, and so adding a fourth facility does not change
/// every call site that already takes one.
///
/// # What belongs here
///
/// Facilities that are *correct to differ per host* and that no pure crate can
/// decide for itself. Not a convenience bag: anything Longhorn can answer from
/// its own rules stays a Longhorn rule.
pub trait HostServices {
    /// Returns a fresh request identifier.
    ///
    /// Used for idempotency and correlation, so it must be unique per call
    /// within one installation's lifetime. Uniqueness is the host's promise;
    /// nothing here checks it.
    fn new_request_id(&self) -> String;

    /// Renders a Unix timestamp as a date a person can read.
    ///
    /// Seconds rather than a `Timestamp` newtype, because this trait sits
    /// below every domain that owns one. Formatting and locale are entirely
    /// the host's business — this returns whatever the host thinks a date
    /// looks like, and nothing compares two of them.
    fn format_timestamp(&self, unix_seconds: i64) -> String;

    /// Folds case for matching, not for display.
    ///
    /// Search and filtering only. Locale matters: Turkish dotless i folds
    /// differently from the Unicode default, so a host that ships in a locale
    /// with special casing rules supplies them here rather than having them
    /// guessed. The result is never shown to anyone.
    fn fold_case(&self, value: &str) -> String;
}

/// Host services for tests, tools, and anything with no locale of its own.
///
/// Deliberately not a `Default` implementation on some other type and
/// deliberately not the fallback when a host supplies nothing: a real
/// application that reaches for this is telling its users that dates look
/// like integers. It exists so a test does not have to invent one.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlainHostServices {
    counter: u64,
}

impl PlainHostServices {
    /// Builds plain services with request ids counting from `first`.
    #[must_use]
    pub const fn new(first: u64) -> Self {
        Self { counter: first }
    }
}

impl HostServices for PlainHostServices {
    fn new_request_id(&self) -> String {
        // Deliberately not random: a test that wants a stable id gets one, and
        // a real host was always going to override this.
        format!("plain:{}", self.counter)
    }

    fn format_timestamp(&self, unix_seconds: i64) -> String {
        unix_seconds.to_string()
    }

    fn fold_case(&self, value: &str) -> String {
        value.to_lowercase()
    }
}

impl fmt::Display for PlainHostServices {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("plain host services")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_services_fold_case_without_a_locale() {
        let services = PlainHostServices::default();
        assert_eq!(services.fold_case("MiXeD"), "mixed");
    }

    #[test]
    fn plain_services_format_a_timestamp_as_the_integer_it_is() {
        // The point of the name: nothing here pretends to be a date.
        let services = PlainHostServices::default();
        assert_eq!(services.format_timestamp(1_700_000_000), "1700000000");
    }
}
