//! Release resolution and bounded artifact download (contract 027, part 4b).
//!
//! [`ReleaseSource`] resolves one release of a repository and downloads one
//! of its assets into a caller's directory. [`HttpReleases`] is the GitHub
//! implementation. The caller supplies every value: the owner, the repository,
//! an optional pinned tag, an optional version floor, the endpoints, the user
//! agent, the bounds and an optional API token. Catalogue entries, pins and
//! asset names stay with the caller.
//!
//! The bytes a download writes are the bytes the server sent. Verifying them
//! is [`crate::verify_artifact`]'s job.

mod failure;
mod http;

use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Url;

pub use failure::{ReleaseError, ReleaseFault, ReleaseRefusal};
pub use http::HttpReleases;

/// Where the release listing and the artifact host are. Both are `https`
/// URLs, except for the loopback fixture constructor (feature
/// `fixture-server`).
#[derive(Clone, Debug)]
pub struct ReleaseEndpoints {
    api: Url,
    download: Url,
    /// Plain `http` is allowed for loopback fixtures only.
    plain_http: bool,
}

impl ReleaseEndpoints {
    /// The public GitHub endpoints: `https://api.github.com` for the release
    /// listing and `https://github.com` for asset downloads.
    pub fn github() -> Self {
        Self {
            api: Url::parse("https://api.github.com").expect("the GitHub API URL is valid"),
            download: Url::parse("https://github.com").expect("the GitHub download URL is valid"),
            plain_http: false,
        }
    }

    /// Endpoints from two `https` base URLs. Anything else is a
    /// [`ReleaseFault::Usage`]: production never talks plain HTTP.
    ///
    /// # Errors
    ///
    /// Returns a usage fault when either base is not a parseable `https` URL.
    pub fn new(api: &str, download: &str) -> Result<Self, ReleaseError> {
        Ok(Self {
            api: parse_base("the API endpoint", api, false)?,
            download: parse_base("the download endpoint", download, false)?,
            plain_http: false,
        })
    }

    /// Endpoints for tests: `http` or `https` on a loopback host (`127.0.0.1`
    /// or `localhost`) only. Only with feature `fixture-server`.
    ///
    /// # Errors
    ///
    /// Returns a usage fault when either base is not a loopback URL.
    #[cfg(feature = "fixture-server")]
    pub fn loopback_for_tests(api: &str, download: &str) -> Result<Self, ReleaseError> {
        Ok(Self {
            api: parse_base("the API endpoint", api, true)?,
            download: parse_base("the download endpoint", download, true)?,
            plain_http: true,
        })
    }

    pub(crate) fn api_base(&self) -> String {
        base_string(&self.api)
    }

    pub(crate) fn download_base(&self) -> String {
        base_string(&self.download)
    }
}

/// Parses one base URL. Production bases are `https` only; a loopback test
/// base may also be `http`, on `127.0.0.1` or `localhost` only.
fn parse_base(label: &str, raw: &str, loopback: bool) -> Result<Url, ReleaseError> {
    let usage = |detail: String| ReleaseError::Fault(ReleaseFault::Usage { detail });
    let url =
        Url::parse(raw).map_err(|error| usage(format!("{label} is not a valid URL: {error}")))?;
    let scheme_ok = match url.scheme() {
        "https" => true,
        "http" => loopback,
        _ => false,
    };
    if !scheme_ok {
        return Err(usage(format!("{label} must be an https URL")));
    }
    if !url.has_host() || url.query().is_some() || url.fragment().is_some() {
        return Err(usage(format!(
            "{label} must name a host and no query or fragment"
        )));
    }
    if loopback && !is_loopback(&url) {
        return Err(usage(format!(
            "{label} must be a loopback host in a test endpoint"
        )));
    }
    Ok(url)
}

fn is_loopback(url: &Url) -> bool {
    matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
}

fn base_string(url: &Url) -> String {
    url.as_str().trim_end_matches('/').to_owned()
}

/// What to resolve. A pinned tag resolves exactly that tag; without one the
/// repository's stable `latest` release is resolved.
#[derive(Clone, Copy, Debug)]
pub struct ReleaseQuery<'a> {
    /// The repository owner, as GitHub names it.
    pub owner: &'a str,
    /// The repository name.
    pub repository: &'a str,
    /// The exact tag to install (for example `v1.4.0`), or `None` to track
    /// `latest`.
    pub pinned_tag: Option<&'a str>,
    /// The lowest version the caller accepts, or `None` for no floor.
    pub version_floor: Option<&'a semver::Version>,
}

/// One resolved release. Only [`ReleaseSource::resolve_release`] makes one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    owner: String,
    repository: String,
    tag: String,
    version: Option<semver::Version>,
    asset_names: Vec<String>,
}

impl Release {
    /// The repository owner the release came from.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The repository name the release came from.
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// The release's tag, exactly as the repository names it.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The version the tag names, or `None` when the tag is not a version
    /// (see [`version_from_tag`]). A floor refuses a release with no version.
    pub fn version(&self) -> Option<&semver::Version> {
        self.version.as_ref()
    }

    /// Every asset name the release lists.
    pub fn asset_names(&self) -> &[String] {
        &self.asset_names
    }

    /// Whether the release lists an asset with this exact name.
    pub fn has_asset(&self, name: &str) -> bool {
        self.asset_names.iter().any(|asset| asset == name)
    }
}

/// A release source: resolves one release and downloads its assets.
pub trait ReleaseSource {
    /// Resolves the release to install. The answer is never a draft or a
    /// pre-release. A pinned tag must be the tag the listing names, exactly.
    /// A version floor refuses a release below it, or with no version.
    ///
    /// # Errors
    ///
    /// Returns a refusal for a release the query does not accept, or a fault
    /// when the source could not answer (see [`ReleaseError`]).
    fn resolve_release(&self, query: &ReleaseQuery<'_>) -> Result<Release, ReleaseError>;

    /// Downloads one asset of `release` into `destination_directory` and
    /// returns its path. The file is created new: an existing file at that
    /// path is an I/O fault. A failed download leaves no partial file.
    ///
    /// # Errors
    ///
    /// Returns a refusal when the release does not list the asset or the
    /// declared or real length exceeds the bound, or a fault for a transport,
    /// rate-limit or file failure.
    fn download_asset(
        &self,
        release: &Release,
        asset: &str,
        destination_directory: &Path,
    ) -> Result<PathBuf, ReleaseError>;
}

/// The bounds and identity of a release source. Every value is the caller's.
#[derive(Clone, Debug)]
pub struct ReleaseOptions {
    user_agent: String,
    timeout: Duration,
    max_artifact_bytes: u64,
}

impl ReleaseOptions {
    /// The default request timeout: 60 seconds for the whole request,
    /// including the response body.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

    /// The default artifact bound: 2 GiB of bytes the server sent.
    pub const DEFAULT_MAX_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

    /// Options with the default timeout and bound and the given user agent.
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self {
            user_agent: user_agent.into(),
            timeout: Self::DEFAULT_TIMEOUT,
            max_artifact_bytes: Self::DEFAULT_MAX_ARTIFACT_BYTES,
        }
    }

    /// Replaces the request timeout. It bounds the whole transfer, so a bound
    /// raised for a large artifact needs a timeout that can finish it too.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Replaces the artifact bound. It counts the bytes the server sent.
    #[must_use]
    pub fn with_max_artifact_bytes(mut self, max_artifact_bytes: u64) -> Self {
        self.max_artifact_bytes = max_artifact_bytes;
        self
    }
}

/// `v1.2.3`, `1.2.3` and `rust-v1.2.3` all name 1.2.3. Only those three forms
/// name a version: one optional prefix, then a semver version, with no
/// trimming. Anything else, such as `vv1.2.3` or `release-1.2.3`, names none.
pub fn version_from_tag(tag: &str) -> Option<semver::Version> {
    let version = tag
        .strip_prefix("rust-v")
        .or_else(|| tag.strip_prefix('v'))
        .unwrap_or(tag);
    semver::Version::parse(version).ok()
}

/// Builds a [`Release`] inside the crate. Keeps the fields private to it.
fn release_from_listing(
    owner: &str,
    repository: &str,
    tag: String,
    asset_names: Vec<String>,
) -> Release {
    Release {
        owner: owner.to_owned(),
        repository: repository.to_owned(),
        version: version_from_tag(&tag),
        tag,
        asset_names,
    }
}

/// A path segment this crate will put in a URL or a file name: ASCII
/// letters, digits, `.`, `_`, `-` and `+`, and not `.` or `..`.
pub(crate) fn check_segment(label: &str, value: &str) -> Result<(), ReleaseError> {
    let plain = !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+'));
    if plain {
        Ok(())
    } else {
        Err(ReleaseError::Fault(ReleaseFault::Usage {
            detail: format!("{label} {value:?} is not a name this crate accepts"),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_names_a_version_with_or_without_a_prefix() {
        assert_eq!(
            version_from_tag("v1.2.3"),
            Some(semver::Version::new(1, 2, 3))
        );
        assert_eq!(
            version_from_tag("1.2.3"),
            Some(semver::Version::new(1, 2, 3))
        );
        assert_eq!(
            version_from_tag("rust-v1.2.3"),
            Some(semver::Version::new(1, 2, 3))
        );
        assert_eq!(version_from_tag("nightly"), None);
    }

    #[test]
    fn endpoints_accept_https_and_refuse_plain_http() {
        assert!(ReleaseEndpoints::new("https://api.example.test", "https://example.test").is_ok());
        let error = ReleaseEndpoints::new("http://api.example.test", "https://example.test")
            .expect_err("plain http refuses");
        assert!(matches!(
            error,
            ReleaseError::Fault(ReleaseFault::Usage { .. })
        ));
    }

    #[test]
    fn path_segments_refuse_separators_and_dot_names() {
        assert!(check_segment("tag", "v1.2.3+build").is_ok());
        for bad in ["", ".", "..", "a/b", "a\\b", "a b", "a?b"] {
            assert!(check_segment("tag", bad).is_err(), "{bad:?} should refuse");
        }
    }
}
