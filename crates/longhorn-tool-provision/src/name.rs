//! The one rule for names this crate writes, fetches or looks up.
//!
//! A managed tool name, binary name, release asset and digest asset all pass
//! through these predicates, so the rule has one definition.

/// A plain path segment: ASCII letters, digits, `.`, `_`, `-` and `+`, and
/// not `.` or `..`. Safe in a URL path, a file name and a lookup key.
pub(crate) fn is_plain_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+'))
}

/// A plain segment that does not start with `.`. Hidden names are reserved
/// for the staging, backup and lock files this crate owns.
pub(crate) fn is_managed_file_name(value: &str) -> bool {
    !value.starts_with('.') && is_plain_segment(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_and_dot_names_are_never_managed_file_names() {
        for name in ["example-tool", "tool_1.2+x"] {
            assert!(is_managed_file_name(name), "{name:?} should pass");
        }
        for name in ["", ".", "..", ".hidden", "a/b", "a\\b", "a b"] {
            assert!(!is_managed_file_name(name), "{name:?} should refuse");
        }
    }
}
