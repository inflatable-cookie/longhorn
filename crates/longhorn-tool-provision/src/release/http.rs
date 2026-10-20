//! The GitHub release source over blocking HTTP (contract 027, part 4b).
//!
//! Two clients, because the token and the redirect rules differ. The listing
//! client carries the token, and follows a redirect only to its own origin.
//! The download client never carries the token, and follows a redirect to any
//! `https` target (GitHub moves assets to its object host). Both refuse a
//! non-HTTPS redirect target unless the endpoints are loopback test endpoints.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use reqwest::{StatusCode, Url};
use secrecy::{ExposeSecret as _, SecretString};
use serde::Deserialize;

use super::failure::{ReleaseError, ReleaseFault, ReleaseRefusal};
use super::{
    Release, ReleaseEndpoints, ReleaseOptions, ReleaseQuery, ReleaseSource, check_segment,
    release_from_listing,
};

/// Redirects followed in one request. GitHub redirects a renamed repository
/// and its asset host; a longer chain is refused.
const MAX_REDIRECTS: usize = 10;
/// The copy's read size. The bound is checked against the bytes read, not
/// against this size.
const COPY_CHUNK: usize = 64 * 1024;

/// The GitHub release source. The token, when given, rides only on listing
/// requests to the configured API origin.
#[derive(Debug)]
pub struct HttpReleases {
    endpoints: ReleaseEndpoints,
    api_client: Client,
    download_client: Client,
    api_token: Option<SecretString>,
    max_artifact_bytes: u64,
}

impl HttpReleases {
    /// Builds a source from the endpoints, the options and an optional token.
    /// The token is never logged and never appears in an error or in `Debug`
    /// output.
    ///
    /// # Errors
    ///
    /// Returns a usage fault for an empty user agent, a zero timeout or a zero
    /// bound, and a client fault when the HTTP client cannot be built.
    pub fn new(
        endpoints: ReleaseEndpoints,
        options: ReleaseOptions,
        api_token: Option<SecretString>,
    ) -> Result<Self, ReleaseError> {
        if options.user_agent.trim().is_empty() {
            return Err(usage("the user agent is empty"));
        }
        if options.timeout.is_zero() {
            return Err(usage("the request timeout is zero"));
        }
        if options.max_artifact_bytes == 0 {
            return Err(usage("the artifact bound is zero"));
        }
        let api_client = build_client(
            &options,
            api_policy(endpoints.api.clone(), endpoints.plain_http),
        )?;
        let download_client = build_client(&options, download_policy(endpoints.plain_http))?;
        Ok(Self {
            api_client,
            download_client,
            api_token,
            max_artifact_bytes: options.max_artifact_bytes,
            endpoints,
        })
    }

    /// One listing request. The token rides it, when there is one.
    fn get_api(&self, url: &str) -> Result<Response, ReleaseError> {
        let mut request = self
            .api_client
            .get(url)
            .header("Accept", "application/vnd.github+json");
        if let Some(token) = &self.api_token {
            request = request.bearer_auth(token.expose_secret());
        }
        let response = request.send().map_err(transport)?;
        classify_rate_limit(response)
    }

    /// One asset request, on the download client: no token, ever.
    fn get_download(&self, url: &str) -> Result<Response, ReleaseError> {
        self.download_client
            .get(url)
            .header("Accept", "application/octet-stream")
            .send()
            .map_err(transport)?
            .error_for_status()
            .map_err(transport)
    }
}

impl ReleaseSource for HttpReleases {
    fn resolve_release(&self, query: &ReleaseQuery<'_>) -> Result<Release, ReleaseError> {
        check_segment("owner", query.owner)?;
        check_segment("repository", query.repository)?;
        if let Some(tag) = query.pinned_tag {
            check_segment("tag", tag)?;
        }
        let base = self.endpoints.api_base();
        let url = match query.pinned_tag {
            Some(tag) => format!(
                "{base}/repos/{}/{}/releases/tags/{tag}",
                query.owner, query.repository
            ),
            None => format!(
                "{base}/repos/{}/{}/releases/latest",
                query.owner, query.repository
            ),
        };
        let response = self.get_api(&url)?;
        let requested = query.pinned_tag.map(str::to_owned);
        if response.status() == StatusCode::NOT_FOUND {
            // The named release (or the repository's releases) does not
            // exist: a verdict, not a transport failure.
            return Err(ReleaseError::Refused(ReleaseRefusal::NoCompatibleRelease {
                floor: query.version_floor.cloned(),
                requested,
            }));
        }
        let response = response.error_for_status().map_err(transport)?;
        let body = response.bytes().map_err(transport)?;
        let listing: ApiRelease = serde_json::from_slice(&body).map_err(|error| {
            ReleaseError::Refused(ReleaseRefusal::Malformed {
                detail: error.to_string(),
            })
        })?;
        if listing.draft || listing.prerelease {
            // Neither endpoint should serve a draft or pre-release, but a pinned
            // tag could. Refuse rather than install one.
            return Err(ReleaseError::Refused(ReleaseRefusal::NoCompatibleRelease {
                floor: query.version_floor.cloned(),
                requested,
            }));
        }
        if let Some(pinned) = query.pinned_tag
            && listing.tag_name != pinned
        {
            // A pinned tool installs exactly the tag it pinned. A different
            // answer, even a newer release, is refused rather than installed.
            return Err(ReleaseError::Refused(ReleaseRefusal::TagMismatch {
                requested: pinned.to_owned(),
                found: listing.tag_name,
            }));
        }
        if check_segment("tag", &listing.tag_name).is_err() {
            return Err(ReleaseError::Refused(ReleaseRefusal::Malformed {
                detail: format!(
                    "the tag {:?} is not a name this crate accepts",
                    listing.tag_name
                ),
            }));
        }
        let release = release_from_listing(
            query.owner,
            query.repository,
            listing.tag_name,
            listing.assets.into_iter().map(|asset| asset.name).collect(),
        );
        if let Some(floor) = query.version_floor
            && release.version().is_none_or(|version| version < floor)
        {
            // A release below the floor, or with no version at all, is refused
            // truthfully rather than paged around.
            return Err(ReleaseError::Refused(ReleaseRefusal::NoCompatibleRelease {
                floor: Some(floor.clone()),
                requested: None,
            }));
        }
        Ok(release)
    }

    fn download_asset(
        &self,
        release: &Release,
        asset: &str,
        destination_directory: &Path,
    ) -> Result<PathBuf, ReleaseError> {
        check_segment("asset", asset)?;
        if !release.has_asset(asset) {
            return Err(ReleaseError::Refused(ReleaseRefusal::MissingAsset {
                release: release.tag().to_owned(),
                asset: asset.to_owned(),
            }));
        }
        let url = format!(
            "{}/{}/{}/releases/download/{}/{asset}",
            self.endpoints.download_base(),
            release.owner(),
            release.repository(),
            release.tag(),
        );
        let mut response = self.get_download(&url)?;
        let limit = self.max_artifact_bytes;
        if let Some(declared) = response.content_length()
            && declared > limit
        {
            // Refused before any byte is written: the declared length is a
            // claim, and the bound is on what the server sends.
            return Err(ReleaseError::Refused(ReleaseRefusal::ArtifactTooLarge {
                asset: asset.to_owned(),
                limit,
            }));
        }
        let destination = destination_directory.join(asset);
        // Created new: a file already at the path is an I/O fault, never
        // overwritten.
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| io_fault(&destination, &error))?;
        let copied = copy_bounded(&mut response, &mut file, limit);
        drop(file);
        match copied {
            Ok(()) => Ok(destination),
            Err(failure) => Err(remove_partial(
                &destination,
                failure.into_error(asset, &destination),
            )),
        }
    }
}

/// The listing's JSON, as far as the release source reads it.
#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
}

fn usage(detail: &str) -> ReleaseError {
    ReleaseError::Fault(ReleaseFault::Usage {
        detail: detail.to_owned(),
    })
}

fn build_client(options: &ReleaseOptions, policy: Policy) -> Result<Client, ReleaseError> {
    Client::builder()
        .timeout(options.timeout)
        .user_agent(options.user_agent.as_str())
        .redirect(policy)
        .build()
        .map_err(|error| {
            ReleaseError::Fault(ReleaseFault::Client {
                detail: error.to_string(),
            })
        })
}

/// A transport error. A redirect the policy refused is a verdict on the
/// server's answer; anything else is a fault.
fn transport(error: reqwest::Error) -> ReleaseError {
    if error.is_redirect() {
        ReleaseError::Refused(ReleaseRefusal::UnsafeRedirect {
            detail: error.to_string(),
        })
    } else {
        ReleaseError::Fault(ReleaseFault::Unreachable {
            detail: error.to_string(),
        })
    }
}

fn io_fault(path: &Path, error: &std::io::Error) -> ReleaseError {
    ReleaseError::Fault(ReleaseFault::Io {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })
}

/// GitHub answers an exhausted quota with 403 (historically) or 429,
/// `X-RateLimit-Remaining: 0` and `X-RateLimit-Reset` in unix time. Paging
/// harder only burns more quota, so this is never retried inside one call.
fn classify_rate_limit(response: Response) -> Result<Response, ReleaseError> {
    let status = response.status();
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
    };
    let limited = (status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS)
        && header("x-ratelimit-remaining") == Some("0");
    if limited {
        let reset_unix = header("x-ratelimit-reset").and_then(|value| value.parse::<i64>().ok());
        return Err(ReleaseError::Fault(ReleaseFault::RateLimited {
            reset_unix,
        }));
    }
    Ok(response)
}

/// Listing redirects: same origin only, https (or loopback http in tests).
/// The token rides these requests, so it never leaves the origin.
fn api_policy(api: Url, plain_http: bool) -> Policy {
    Policy::custom(move |attempt| {
        match redirect_verdict(
            attempt.url(),
            attempt.previous().len(),
            Some(&api),
            plain_http,
        ) {
            Ok(()) => attempt.follow(),
            Err(detail) => attempt.error(detail),
        }
    })
}

/// Asset redirects: any https target. No token rides them.
fn download_policy(plain_http: bool) -> Policy {
    Policy::custom(move |attempt| {
        match redirect_verdict(attempt.url(), attempt.previous().len(), None, plain_http) {
            Ok(()) => attempt.follow(),
            Err(detail) => attempt.error(detail),
        }
    })
}

/// The redirect rule as a pure decision. `same_origin` names the origin a
/// redirect must stay on, when the request has one.
fn redirect_verdict(
    target: &Url,
    hops: usize,
    same_origin: Option<&Url>,
    plain_http: bool,
) -> Result<(), String> {
    if hops >= MAX_REDIRECTS {
        return Err("too many redirects".to_owned());
    }
    let scheme_allowed = match target.scheme() {
        "https" => true,
        "http" => plain_http,
        _ => false,
    };
    if !scheme_allowed {
        return Err(format!(
            "the redirect target uses {}, not https",
            target.scheme()
        ));
    }
    if let Some(origin) = same_origin
        && target.origin() != origin.origin()
    {
        return Err("the listing redirected to another origin".to_owned());
    }
    Ok(())
}

/// Why a body copy stopped. The variant decides the class.
enum CopyFailure {
    /// Reading the response failed: a transport or truncation fault.
    Source(std::io::Error),
    /// Writing the destination failed: a host fault.
    Sink(std::io::Error),
    /// The bytes the server sent passed the bound.
    TooLarge(u64),
}

impl CopyFailure {
    fn into_error(self, asset: &str, destination: &Path) -> ReleaseError {
        match self {
            Self::Source(error) => ReleaseError::Fault(ReleaseFault::Unreachable {
                detail: format!("{asset}: {error}"),
            }),
            Self::Sink(error) => io_fault(destination, &error),
            Self::TooLarge(limit) => ReleaseError::Refused(ReleaseRefusal::ArtifactTooLarge {
                asset: asset.to_owned(),
                limit,
            }),
        }
    }
}

/// Copies the source into the sink and stops at `limit + 1` bytes. Counts the
/// bytes read, whatever the response declared.
fn copy_bounded(
    source: &mut impl Read,
    sink: &mut impl Write,
    limit: u64,
) -> Result<(), CopyFailure> {
    let mut bounded = source.by_ref().take(limit.saturating_add(1));
    let mut buffer = vec![0u8; COPY_CHUNK];
    let mut copied: u64 = 0;
    loop {
        let read = match bounded.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(CopyFailure::Source(error)),
        };
        if read == 0 {
            return Ok(());
        }
        copied += read as u64;
        if copied > limit {
            return Err(CopyFailure::TooLarge(limit));
        }
        sink.write_all(&buffer[..read]).map_err(CopyFailure::Sink)?;
    }
}

/// Removes the partial file a failed download left. A removal that also fails
/// is a host fault that keeps the original failure in its detail.
fn remove_partial(destination: &Path, failure: ReleaseError) -> ReleaseError {
    match std::fs::remove_file(destination) {
        Ok(()) => failure,
        Err(error) if error.kind() == ErrorKind::NotFound => failure,
        Err(removal) => ReleaseError::Fault(ReleaseFault::Io {
            path: destination.to_path_buf(),
            detail: format!("{failure}; removing the partial file also failed: {removal}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(raw: &str) -> Url {
        Url::parse(raw).expect("test URL parses")
    }

    #[test]
    fn a_listing_redirect_stays_on_its_origin() {
        let api = url("https://api.example.test");
        assert!(
            redirect_verdict(
                &url("https://api.example.test/repos/x"),
                0,
                Some(&api),
                false
            )
            .is_ok()
        );
        assert!(
            redirect_verdict(
                &url("https://other.example.test/repos/x"),
                0,
                Some(&api),
                false
            )
            .is_err()
        );
        assert!(
            redirect_verdict(
                &url("https://api.example.test:444/repos/x"),
                0,
                Some(&api),
                false
            )
            .is_err()
        );
    }

    #[test]
    fn a_redirect_to_plain_http_is_refused_outside_loopback_tests() {
        let asset = url("https://github.example.test/a");
        assert!(redirect_verdict(&url("http://127.0.0.1:9/a"), 0, None, false).is_err());
        assert!(redirect_verdict(&url("http://127.0.0.1:9/a"), 0, None, true).is_ok());
        assert!(redirect_verdict(&url("https://objects.example.test/a"), 0, None, false).is_ok());
        assert!(redirect_verdict(&asset, MAX_REDIRECTS, None, false).is_err());
    }
}
