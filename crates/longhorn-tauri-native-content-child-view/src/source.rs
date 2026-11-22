//! Bounded initial document sources for one child island.
//!
//! The carrier is deliberately narrow: a validated HTTPS or loopback HTTP URL or a
//! validated relative path inside the host application's bundled assets.
//! Arbitrary filesystem URLs, custom schemes, and unvalidated paths are never
//! accepted. Bundled documents are served by the host application's own asset
//! protocol (`WebviewUrl::App` semantics); the concrete app-scheme URL is
//! Tauri-owned and observable through `ChildViewAdapter::current_url`.

use std::fmt;

use longhorn_url::{LoopbackHttp, classify_endpoint};
use tauri::Url;

use crate::ChildViewError;

/// Longest accepted bundled document path, in UTF-8 bytes.
const MAX_BUNDLED_DOCUMENT_BYTES: usize = 512;

/// One validated relative document path inside the host application's bundled
/// assets.
///
/// Validation is the security boundary, so the rules are exact:
///
/// - non-empty and at most 512 UTF-8 bytes;
/// - relative: no leading or trailing slash and no empty components;
/// - confined: no `.` or `..` component, so the path cannot escape the
///   bundled asset root;
/// - conservative grammar: only ASCII alphanumerics and `-`, `_`, `.`, `/`.
///   This excludes backslash, colon, percent, query, fragment, and control
///   characters, so the path can neither inject a URL scheme nor shift
///   meaning through URL syntax; it maps one-to-one onto the asset key.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BundledLocalDocument(String);

impl BundledLocalDocument {
    /// Validates one bounded bundled document path.
    ///
    /// # Errors
    ///
    /// Returns [`ChildViewError::InvalidBundledContentSource`] when the path
    /// breaks any rule in the type documentation.
    pub fn new(path: impl Into<String>) -> Result<Self, ChildViewError> {
        let path = path.into();
        if path.is_empty()
            || path.len() > MAX_BUNDLED_DOCUMENT_BYTES
            || path.starts_with('/')
            || path.ends_with('/')
            || path.contains("//")
            || !path
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_./".contains(&byte))
            || path
                .split('/')
                .any(|component| component.is_empty() || component == "." || component == "..")
        {
            return Err(ChildViewError::InvalidBundledContentSource);
        }
        Ok(Self(path))
    }

    /// Returns the exact validated asset path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BundledLocalDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The bounded initial document source for one child island.
///
/// Construct through [`ChildViewSource::external`] or
/// [`ChildViewSource::bundled`]; [`ChildViewSpec`](crate::ChildViewSpec) re- validates
/// external URLs at construction, and bundled paths carry their validation in
/// [`BundledLocalDocument`].
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChildViewSource {
    /// An HTTPS document or an HTTP document hosted on loopback.
    External(Url),
    /// A bundled local document served by the host application's own asset
    /// protocol. The document is part of the application bundle: no local
    /// server, no arbitrary filesystem access, and no network dependency.
    Bundled(BundledLocalDocument),
}

impl ChildViewSource {
    /// Validates and wraps one HTTPS or loopback HTTP document URL.
    ///
    /// # Errors
    ///
    /// Returns [`ChildViewError::InvalidContentSource`] unless the URL names
    /// an HTTPS endpoint or a loopback HTTP endpoint.
    pub fn external(url: Url) -> Result<Self, ChildViewError> {
        Self::validate_external(&url)?;
        Ok(Self::External(url))
    }

    pub(crate) fn validate_external(url: &Url) -> Result<(), ChildViewError> {
        classify_endpoint(url.as_str(), LoopbackHttp::Allowed)
            .map(|_| ())
            .map_err(|_| ChildViewError::InvalidContentSource)
    }

    /// Validates and wraps one bundled document path.
    ///
    /// # Errors
    ///
    /// Returns [`ChildViewError::InvalidBundledContentSource`] when the path
    /// breaks any [`BundledLocalDocument`] rule.
    pub fn bundled(path: impl Into<String>) -> Result<Self, ChildViewError> {
        Ok(Self::Bundled(BundledLocalDocument::new(path)?))
    }
}

impl fmt::Display for ChildViewSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::External(url) => url.fmt(formatter),
            Self::Bundled(document) => write!(formatter, "bundled:{document}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BundledLocalDocument, ChildViewError, ChildViewSource};

    #[test]
    fn bundled_documents_admit_bounded_confined_paths() {
        for path in [
            "document.html",
            "docs/offline/index.htm",
            "a0-._/A9-zZ.bundled",
        ] {
            let document = BundledLocalDocument::new(path).unwrap();
            assert_eq!(document.as_str(), path);
        }
    }

    #[test]
    fn bundled_documents_reject_escape_and_syntax() {
        for path in [
            "",
            "/document.html",
            "document.html/",
            "a//b",
            "..",
            "docs/../escape.html",
            "docs/./escape.html",
            ".",
            "a\\b",
            "https://escape.invalid/x",
            "document.html?query=1",
            "document.html#fragment",
            "document%2ehtml",
            "doc ument.html",
            "do\0cument.html",
            "é.html",
            "a".repeat(513).as_str(),
        ] {
            assert_eq!(
                BundledLocalDocument::new(path),
                Err(ChildViewError::InvalidBundledContentSource),
                "path {path:?} must be rejected"
            );
        }
    }

    #[test]
    fn external_sources_stay_remote_and_hosted() {
        assert!(ChildViewSource::external("https://example.com/page".parse().unwrap()).is_ok());
        assert!(ChildViewSource::external("http://127.0.0.1:43119/proof".parse().unwrap()).is_ok());
        assert_eq!(
            ChildViewSource::external("http://example.com/page".parse().unwrap()),
            Err(ChildViewError::InvalidContentSource)
        );
        assert_eq!(
            ChildViewSource::external("file:///etc/passwd".parse().unwrap()),
            Err(ChildViewError::InvalidContentSource)
        );
        assert_eq!(
            ChildViewSource::external("ftp://example.com/x".parse().unwrap()),
            Err(ChildViewError::InvalidContentSource)
        );
        assert_eq!(
            ChildViewSource::external("tauri://localhost/index.html".parse().unwrap()),
            Err(ChildViewError::InvalidContentSource)
        );
    }

    #[test]
    fn bundled_sources_render_a_stable_identity() {
        let source = ChildViewSource::bundled("docs/offline/index.html").unwrap();
        assert_eq!(source.to_string(), "bundled:docs/offline/index.html");
        assert_eq!(
            ChildViewSource::external("https://example.com/page".parse().unwrap())
                .unwrap()
                .to_string(),
            "https://example.com/page"
        );
    }
}
