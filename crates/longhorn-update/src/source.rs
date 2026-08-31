use core::fmt;
use std::error::Error;

use longhorn_url::{EndpointClassificationError, LoopbackHttp, classify_endpoint};
use serde::{Deserialize, Serialize};

use crate::{Artifact, Channel};

/// A URL an update request may be issued against.
///
/// HTTPS is required, with one exception: plain HTTP is accepted for
/// loopback, which is what a local shim endpoint needs. Artifacts are
/// signature-verified by the installer regardless, but a manifest is not —
/// a tampered manifest cannot forge an artifact, though it can withhold one
/// or pin an install to a stale version, so the transport still matters.
///
/// Scheme and loopback parsing live in `longhorn-url`; this newtype keeps the
/// update-domain error vocabulary.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct EndpointUrl(String);

impl From<EndpointUrl> for String {
    fn from(value: EndpointUrl) -> Self {
        value.0
    }
}

impl EndpointUrl {
    /// Validates and records a URL.
    pub fn new(value: impl Into<String>) -> Result<Self, EndpointUrlError> {
        let value = value.into();
        match classify_endpoint(&value, LoopbackHttp::Allowed) {
            Ok(_) => Ok(Self(value)),
            Err(EndpointClassificationError::UnsupportedScheme) => {
                Err(EndpointUrlError::UnsupportedScheme)
            }
            Err(EndpointClassificationError::InsecureScheme) => {
                Err(EndpointUrlError::InsecureScheme)
            }
            Err(EndpointClassificationError::MissingHost) => Err(EndpointUrlError::MissingHost),
        }
    }

    /// Returns the URL.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EndpointUrl {
    type Error = EndpointUrlError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for EndpointUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Endpoint URL validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointUrlError {
    /// The scheme was neither HTTPS nor loopback HTTP.
    UnsupportedScheme,
    /// Plain HTTP was used for a non-loopback host.
    InsecureScheme,
    /// No host followed the scheme.
    MissingHost,
}

impl fmt::Display for EndpointUrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedScheme => "endpoint must be https, or http on loopback",
            Self::InsecureScheme => "plain http is only accepted for loopback hosts",
            Self::MissingHost => "endpoint has no host",
        })
    }
}

impl Error for EndpointUrlError {}

/// One HTTP request an update needs, as a description rather than a call.
///
/// This crate never performs I/O. A request is handed to the Tauri updater,
/// which fetches, verifies, and installs — so an adapter's whole job is to
/// say *where* and *with what headers*.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRequest {
    /// Where to send the request.
    pub url: EndpointUrl,
    /// Headers to send with it.
    ///
    /// Present because authenticated hosts cannot express credentials in a
    /// URL alone: a private GitHub asset needs an `Accept` header and a
    /// bearer token, where a presigned S3 URL needs nothing.
    pub headers: Vec<(String, String)>,
}

impl SourceRequest {
    /// Records an unauthenticated request.
    #[must_use]
    pub const fn new(url: EndpointUrl) -> Self {
        Self {
            url,
            headers: Vec::new(),
        }
    }

    /// Adds one header.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

/// Describes where a channel's manifest and artifacts come from.
///
/// Implementors describe requests; they never fetch, verify, or install, and
/// they have no authority to weaken policy. A source that cannot be reached
/// yields no update — never a degraded or unverified one.
pub trait UpdateSource {
    /// Returns the request that retrieves one channel's manifest.
    fn manifest_request(&self, channel: Channel) -> Result<SourceRequest, SourceError>;

    /// Returns the request that retrieves one artifact.
    ///
    /// Defaults to fetching the artifact's own URL unauthenticated, which is
    /// correct for every public host and for presigned URLs.
    fn artifact_request(&self, artifact: &Artifact) -> Result<SourceRequest, SourceError> {
        Ok(SourceRequest::new(EndpointUrl::new(&artifact.url)?))
    }
}

/// Adapter failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceError {
    /// A composed URL was not usable.
    Url(EndpointUrlError),
    /// The adapter cannot serve this channel.
    UnsupportedChannel {
        /// The channel asked for.
        channel: Channel,
    },
}

impl From<EndpointUrlError> for SourceError {
    fn from(value: EndpointUrlError) -> Self {
        Self::Url(value)
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(error) => write!(formatter, "unusable endpoint: {error}"),
            Self::UnsupportedChannel { channel } => {
                write!(formatter, "source does not serve the {channel} channel")
            }
        }
    }
}

impl Error for SourceError {}

/// A manifest per channel under one base URL.
///
/// The reference shape, and what every other adapter degrades to. Suits any
/// static host: object storage, a CDN, or a plain web server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticJsonSource {
    base: String,
}

impl StaticJsonSource {
    /// Records a source serving `<base>/<channel>.json`.
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
        }
    }
}

impl UpdateSource for StaticJsonSource {
    fn manifest_request(&self, channel: Channel) -> Result<SourceRequest, SourceError> {
        Ok(SourceRequest::new(EndpointUrl::new(format!(
            "{}/{}.json",
            self.base,
            channel.as_str()
        ))?))
    }
}

/// A manifest published as a GitHub release asset.
///
/// Covers both the plain case and the separate-public-releases-repository
/// case, which differ only by coordinate: the source repository stays
/// private while a second public repository holds nothing but tags and
/// binaries.
///
/// **Public repositories only.** A private repository's asset URLs cannot be
/// authenticated directly — download needs an asset-ID API call plus an
/// `Accept: application/octet-stream` header, which the released-asset URL
/// cannot carry. The workable route is an authenticating proxy, which is a
/// consumer-implemented `UpdateSource`, not something this adapter can fake.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubReleasesSource {
    owner: String,
    repository: String,
}

impl GitHubReleasesSource {
    /// Records a source over one public repository's latest release.
    #[must_use]
    pub fn new(owner: impl Into<String>, repository: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repository: repository.into(),
        }
    }
}

impl UpdateSource for GitHubReleasesSource {
    fn manifest_request(&self, channel: Channel) -> Result<SourceRequest, SourceError> {
        Ok(SourceRequest::new(EndpointUrl::new(format!(
            "https://github.com/{}/{}/releases/latest/download/{}.json",
            self.owner,
            self.repository,
            channel.as_str()
        ))?))
    }
}

/// A manifest in an S3-compatible bucket, R2 included.
///
/// Presigning is injected rather than implemented: request signing belongs
/// to whichever SDK the consumer already uses, and a pure policy crate is
/// the wrong place for it. Without a presigner this behaves as a static
/// host, which is correct for a public bucket.
pub struct ObjectStorageSource {
    base: String,
    presign: Option<Box<Presigner>>,
}

/// Signs one composed URL. Supplied by the consumer's own SDK.
type Presigner = dyn Fn(&str) -> String + Send + Sync;

impl ObjectStorageSource {
    /// Records a source over a public bucket.
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            presign: None,
        }
    }

    /// Signs every composed URL with the supplied presigner.
    #[must_use]
    pub fn with_presigner<F>(mut self, presign: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.presign = Some(Box::new(presign));
        self
    }

    fn sign(&self, url: String) -> String {
        self.presign
            .as_ref()
            .map_or(url.clone(), |presign| presign(&url))
    }
}

impl fmt::Debug for ObjectStorageSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectStorageSource")
            .field("base", &self.base)
            .field("presigned", &self.presign.is_some())
            .finish()
    }
}

impl UpdateSource for ObjectStorageSource {
    fn manifest_request(&self, channel: Channel) -> Result<SourceRequest, SourceError> {
        let url = self.sign(format!("{}/{}.json", self.base, channel.as_str()));
        Ok(SourceRequest::new(EndpointUrl::new(url)?))
    }

    fn artifact_request(&self, artifact: &Artifact) -> Result<SourceRequest, SourceError> {
        let url = self.sign(artifact.url.clone());
        Ok(SourceRequest::new(EndpointUrl::new(url)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_is_accepted_and_plain_http_is_not() {
        assert!(EndpointUrl::new("https://updates.example.com/x.json").is_ok());
        assert_eq!(
            EndpointUrl::new("http://updates.example.com/x.json"),
            Err(EndpointUrlError::InsecureScheme)
        );
        assert_eq!(
            EndpointUrl::new("ftp://updates.example.com"),
            Err(EndpointUrlError::UnsupportedScheme)
        );
        assert_eq!(
            EndpointUrl::new("https://"),
            Err(EndpointUrlError::MissingHost)
        );
    }

    #[test]
    fn loopback_http_is_accepted_for_the_local_shim() {
        for url in [
            "http://127.0.0.1:8000/production.json",
            "http://localhost:9/x",
            "http://[::1]:80/x",
        ] {
            assert!(EndpointUrl::new(url).is_ok(), "{url} should be accepted");
        }
    }

    #[test]
    fn a_host_merely_starting_with_localhost_is_not_loopback() {
        // `localhost.example.com` is a remote host. Prefix matching here
        // would open plain HTTP to anyone who can register that name.
        assert_eq!(
            EndpointUrl::new("http://localhost.example.com/x.json"),
            Err(EndpointUrlError::InsecureScheme)
        );
        assert_eq!(
            EndpointUrl::new("http://127.0.0.1.example.com/x.json"),
            Err(EndpointUrlError::InsecureScheme)
        );
    }

    #[test]
    fn userinfo_cannot_make_a_remote_host_look_like_loopback() {
        // The host begins after the last '@'. Parsing the authority without
        // stripping userinfo would accept these as loopback while the fetch
        // goes to a remote host over plaintext.
        for url in [
            "http://127.0.0.1:80@evil.example/x",
            "http://[::1]@evil.example/",
            "http://localhost@evil.example/x",
            "http://@evil.example/x",
        ] {
            assert_eq!(
                EndpointUrl::new(url),
                Err(EndpointUrlError::InsecureScheme),
                "{url} should be refused"
            );
        }
    }

    #[test]
    fn a_backslash_cannot_push_the_host_past_the_authority() {
        // WHATWG ends the authority at '\' for special schemes, so these
        // fetch from `evil.example`. Splitting only on '/' would leave
        // `evil.example\` as userinfo and read the tail as a loopback host.
        for url in [
            r"http://evil.example\@127.0.0.1/x",
            r"http://evil.example\@localhost/x",
            r"http://evil.example\@[::1]/x",
            r"http://evil.example\127.0.0.1",
        ] {
            assert_eq!(
                EndpointUrl::new(url),
                Err(EndpointUrlError::InsecureScheme),
                "{url} should be refused"
            );
        }
    }

    #[test]
    fn a_backslash_after_a_loopback_authority_is_still_loopback() {
        // The mirror case: the authority genuinely is loopback and the
        // backslash begins the path. A conforming parser agrees, so refusing
        // here would reject a legitimate local endpoint.
        for url in [r"http://127.0.0.1\@evil.example", r"http://localhost:9\x"] {
            assert!(EndpointUrl::new(url).is_ok(), "{url} should be accepted");
        }
    }

    #[test]
    fn userinfo_on_a_genuine_loopback_host_is_still_loopback() {
        // Credentials in the authority are unusual but legal; the exception
        // binds to the host, not to the absence of userinfo.
        for url in [
            "http://user@127.0.0.1:8000/x",
            "http://user:pass@localhost:9/x",
            "http://evil.example@127.0.0.1/x",
            "http://user@[::1]:80/x",
        ] {
            assert!(EndpointUrl::new(url).is_ok(), "{url} should be accepted");
        }
    }

    #[test]
    fn the_static_source_composes_one_url_per_channel() {
        let source = StaticJsonSource::new("https://updates.example.com/");

        assert_eq!(
            source
                .manifest_request(Channel::Nightly)
                .unwrap()
                .url
                .as_str(),
            "https://updates.example.com/nightly.json"
        );
        assert_eq!(
            source
                .manifest_request(Channel::Production)
                .unwrap()
                .url
                .as_str(),
            "https://updates.example.com/production.json"
        );
    }

    #[test]
    fn the_github_source_addresses_the_latest_release_asset() {
        let source = GitHubReleasesSource::new("inflatable-cookie", "example-releases");

        assert_eq!(
            source.manifest_request(Channel::Beta).unwrap().url.as_str(),
            "https://github.com/inflatable-cookie/example-releases/releases/latest/download/beta.json"
        );
    }

    #[test]
    fn an_unpresigned_bucket_behaves_as_a_static_host() {
        let source = ObjectStorageSource::new("https://bucket.example.com");
        let artifact = Artifact::new("https://bucket.example.com/app.tar.gz", "sig");

        assert_eq!(
            source
                .manifest_request(Channel::Production)
                .unwrap()
                .url
                .as_str(),
            "https://bucket.example.com/production.json"
        );
        assert_eq!(
            source.artifact_request(&artifact).unwrap().url.as_str(),
            "https://bucket.example.com/app.tar.gz"
        );
    }

    #[test]
    fn a_presigner_signs_both_manifests_and_artifacts() {
        let source = ObjectStorageSource::new("https://bucket.example.com")
            .with_presigner(|url| format!("{url}?X-Amz-Signature=deadbeef"));
        let artifact = Artifact::new("https://bucket.example.com/app.tar.gz", "sig");

        assert!(
            source
                .manifest_request(Channel::Production)
                .unwrap()
                .url
                .as_str()
                .ends_with("?X-Amz-Signature=deadbeef")
        );
        assert!(
            source
                .artifact_request(&artifact)
                .unwrap()
                .url
                .as_str()
                .ends_with("?X-Amz-Signature=deadbeef")
        );
    }

    #[test]
    fn the_default_artifact_request_uses_the_artifact_url_unauthenticated() {
        let source = StaticJsonSource::new("https://updates.example.com");
        let artifact = Artifact::new("https://cdn.example.com/app.tar.gz", "sig");

        let request = source.artifact_request(&artifact).unwrap();

        assert_eq!(request.url.as_str(), "https://cdn.example.com/app.tar.gz");
        assert!(request.headers.is_empty());
    }

    #[test]
    fn an_unusable_artifact_url_is_refused_rather_than_passed_through() {
        let source = StaticJsonSource::new("https://updates.example.com");
        let artifact = Artifact::new("http://cdn.example.com/app.tar.gz", "sig");

        assert_eq!(
            source.artifact_request(&artifact),
            Err(SourceError::Url(EndpointUrlError::InsecureScheme))
        );
    }

    #[test]
    fn headers_ride_alongside_the_url_for_authenticated_hosts() {
        let request = SourceRequest::new(EndpointUrl::new("https://api.example.com/x").unwrap())
            .with_header("Accept", "application/octet-stream")
            .with_header("Authorization", "Bearer token");

        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers[0].0, "Accept");
    }
}
