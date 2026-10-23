//! GitHub device-flow sign-in, the stored grant and its refresh, sign-out, and
//! the credential-store readings (contract 029, part 5b).
//!
//! The mechanism owns GitHub's device and poll protocol, the stored grant
//! format with its five-minute refresh skew, the serialized single-use
//! refresh, the token-exchange classification, the failure vocabulary and the
//! progress readings. The application owns every value and every sentence: the
//! OAuth client id, the credential store and slot, the endpoints a build uses,
//! the user agent, and all copy.
//!
//! Tokens live in [`secrecy`] types here and in the grant. They are written
//! only to the caller's [`CredentialStore`] slot, reach Git only through the
//! askpass environment ([`crate::GitCredentialBridge`]), and never enter a
//! `Debug` output, an error, or a log line.

use std::{
    fmt,
    io::Read as _,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use longhorn_core::{CredentialError, CredentialSlot, CredentialStore};
use reqwest::blocking::{Client, Response};
use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};

/// GitHub's device-code endpoint. The generic protocol endpoint; which one a
/// build actually uses is the caller's configuration.
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";

/// GitHub's access-token endpoint, used for both the device-flow poll and the
/// refresh exchange.
pub const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// GitHub's device verification page. It is the only verification address the
/// mechanism reports, and the application opens it.
pub const GITHUB_DEVICE_VERIFICATION_URL: &str = "https://github.com/login/device";

/// The poll interval used when the device-code response suggests none.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// The longest the device flow waits for the operator, whatever the
/// device-code response declares.
pub const MAX_DEVICE_WAIT: Duration = Duration::from_secs(900);

/// How long before an access token expires the stored grant is refreshed.
pub const ACCESS_REFRESH_SKEW: Duration = Duration::from_secs(5 * 60);

/// The wall clock one token request is allowed.
pub const TOKEN_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Most bytes read from one token or profile response.
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

/// The stored grant format's version. A value with another version is read as
/// a bare access token, so an unrecognised document is not trusted.
const GRANT_VERSION: u8 = 1;

/// How much one `slow_down` answer lengthens the poll interval when the server
/// suggests nothing.
const SLOW_DOWN_STEP: Duration = Duration::from_secs(5);

/// The cancellation check interval during a poll wait.
const CANCEL_POLL_STEP: Duration = Duration::from_millis(100);

/// Serializes refreshes process-wide: two callers that need one at the same
/// time cause one exchange, and the second adopts the rotated grant.
static REFRESH_LOCK: Mutex<()> = Mutex::new(());

/// What one GitHub sign-in step could not do.
///
/// The application maps each reading to its own copy. The mechanism authors no
/// operator sentence and carries no token, response body or account detail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GithubAuthFailure {
    /// The build's OAuth registration or endpoint configuration was refused.
    NotConfigured,
    /// The credential store holds no grant.
    NotSignedIn,
    /// The credential store could not be read or written.
    CredentialStoreUnavailable,
    /// The token endpoint could not be reached, or answered with a rate limit
    /// or a server fault.
    Offline,
    /// The operator declined the one-time code.
    Denied,
    /// The device flow's wait elapsed, or a token request ran past its own
    /// wall clock.
    TimedOut,
    /// The caller's cancellation flag was set.
    Cancelled,
    /// The operator's browser could not be opened. The mechanism never opens a
    /// browser; the application's own step reports this class into the same
    /// vocabulary.
    BrowserUnavailable,
    /// GitHub answered, but not in a way this step could use.
    Failed,
    /// The refresh token is expired, revoked, or missing on an access token
    /// known to have expired. The operator must sign in again; the stored grant
    /// is left in place until they do.
    SignInExpired,
}

/// Safe progress from the device flow.
///
/// It carries the one-time user code and GitHub's verification page, and
/// nothing else: no device code, no access token, no account value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GithubAuthProgress {
    /// The code is ready to show. The application displays it, and opens
    /// [`verification_uri`](Self::AwaitingVerification::verification_uri) from
    /// the operator's own action.
    AwaitingVerification {
        /// The one-time code the operator enters on the verification page.
        user_code: String,
        /// GitHub's verification page.
        verification_uri: String,
    },
    /// One poll has passed without an answer.
    Polling {
        /// The one-time code the operator enters on the verification page.
        user_code: String,
        /// GitHub's verification page.
        verification_uri: String,
    },
}

/// What the caller's credential store holds in one slot, read without exposing
/// a value.
///
/// The application names these readings; the mechanism authors no copy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialStoreReading {
    /// The slot is empty.
    Missing,
    /// The slot holds a value.
    Present,
    /// The store could not be read. Carries the backend's own words so the
    /// application can keep one safe diagnostic; never a secret.
    Unavailable(String),
}

/// The explicit credential scope for one mechanism call: the caller's store and
/// the exact slot its grant lives in.
///
/// The mechanism never chooses a store or a slot.
pub struct CredentialScope<'a> {
    /// The application's credential store.
    pub store: &'a dyn CredentialStore,
    /// The application's slot for the GitHub grant.
    pub slot: &'a CredentialSlot,
}

/// A stored GitHub grant: one access token, its expiry, and the refresh token
/// that replaces it.
///
/// `Debug` names no token value. Expiries are `None` when the token endpoint
/// declared none, which means the token does not expire on its own.
pub struct TokenGrant {
    /// The access token the mechanism hands to Git through the credential
    /// bridge, and to the application's own API reads.
    pub access_token: SecretString,
    /// When the access token expires, when the token endpoint declared it.
    pub access_expires_at: Option<SystemTime>,
    /// The token that replaces the access token, when GitHub issued one.
    pub refresh_token: Option<SecretString>,
    /// When the refresh token expires, when the token endpoint declared it.
    pub refresh_expires_at: Option<SystemTime>,
}

impl fmt::Debug for TokenGrant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenGrant")
            .field("access_token", &"[redacted]")
            .field("access_expires_at", &self.access_expires_at)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("refresh_expires_at", &self.refresh_expires_at)
            .finish()
    }
}

/// GitHub's device-flow endpoints.
///
/// [`github`](Self::github) is the public protocol pair. [`new`](Self::new)
/// accepts `https` only; the loopback constructor, which accepts plain `http`
/// on `127.0.0.1`, exists behind the `fixture-server` test-support feature.
#[derive(Debug)]
pub struct DeviceFlowEndpoints<'a> {
    device_code_url: &'a str,
    access_token_url: &'a str,
}

impl<'a> DeviceFlowEndpoints<'a> {
    /// GitHub's public device-flow endpoints.
    #[must_use]
    pub fn github() -> Self {
        Self {
            device_code_url: GITHUB_DEVICE_CODE_URL,
            access_token_url: GITHUB_ACCESS_TOKEN_URL,
        }
    }

    /// Endpoints from two `https` URLs. A build that talks plain HTTP fails
    /// here instead.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotConfigured`] when either URL is not a
    /// parseable `https` address.
    pub fn new(
        device_code_url: &'a str,
        access_token_url: &'a str,
    ) -> Result<Self, GithubAuthFailure> {
        require_https(device_code_url)?;
        require_https(access_token_url)?;
        Ok(Self {
            device_code_url,
            access_token_url,
        })
    }

    /// Endpoints for tests: `http` or `https` on a loopback host only.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotConfigured`] when either URL is not a
    /// loopback address with an explicit port.
    #[cfg(feature = "fixture-server")]
    pub fn loopback_for_tests(
        device_code_url: &'a str,
        access_token_url: &'a str,
    ) -> Result<Self, GithubAuthFailure> {
        require_loopback(device_code_url)?;
        require_loopback(access_token_url)?;
        Ok(Self {
            device_code_url,
            access_token_url,
        })
    }
}

/// One token-exchange transport: the exchange the mechanism drives and the
/// caller injects.
///
/// The production implementation is [`GithubTokenClient`]. A caller may supply
/// its own so a build's transport branding, proxy or recording is its own
/// decision.
pub trait TokenClient: Send + Sync {
    /// Posts one form body to the token endpoint and returns GitHub's answer.
    ///
    /// # Errors
    ///
    /// Returns a [`GithubAuthFailure`] when the request could not be made or
    /// its answer could not be read. The classification of GitHub's own error
    /// codes is [`classify_token_exchange`]'s job.
    fn exchange(&self, fields: &[(&str, &str)]) -> Result<AccessTokenResponse, GithubAuthFailure>;
}

/// GitHub's answer to a token request.
///
/// It is the wire shape, so the tokens are plain values here; the mechanism
/// moves them into [`SecretString`] as soon as it builds a [`TokenGrant`], and
/// never prints, logs or returns this structure itself.
#[derive(Clone, Default, Deserialize)]
pub struct AccessTokenResponse {
    /// The access token, when the request succeeded.
    pub access_token: Option<String>,
    /// How many seconds the access token is valid for.
    pub expires_in: Option<u64>,
    /// The token that replaces the access token.
    pub refresh_token: Option<String>,
    /// How many seconds the refresh token is valid for.
    pub refresh_token_expires_in: Option<u64>,
    /// GitHub's machine error code, when the request failed.
    pub error: Option<String>,
    /// The interval the server suggests after a `slow_down`.
    pub interval: Option<u64>,
}

impl fmt::Debug for AccessTokenResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccessTokenResponse")
            .field(
                "access_token",
                &self.access_token.as_ref().map(|_| "[redacted]"),
            )
            .field("expires_in", &self.expires_in)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("refresh_token_expires_in", &self.refresh_token_expires_in)
            // The error field is body text, not a fixed code: an endpoint can
            // echo anything there, including a token. `Debug` shows only that
            // it was present.
            .field("error", &self.error.as_ref().map(|_| "[redacted]"))
            .field("interval", &self.interval)
            .finish()
    }
}

/// The production token client: one blocking `https` request per exchange.
#[derive(Debug)]
pub struct GithubTokenClient {
    url: String,
}

impl GithubTokenClient {
    /// A client for one `https` token endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotConfigured`] when the URL is not a
    /// parseable `https` address.
    pub fn new(url: impl Into<String>) -> Result<Self, GithubAuthFailure> {
        let url = url.into();
        require_https(&url)?;
        Ok(Self { url })
    }

    /// A client for a loopback test server: `http` or `https` on a loopback
    /// host only.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotConfigured`] when the URL is not a
    /// loopback address with an explicit port.
    #[cfg(feature = "fixture-server")]
    pub fn loopback_for_tests(url: &str) -> Result<Self, GithubAuthFailure> {
        require_loopback(url)?;
        Ok(Self {
            url: url.to_owned(),
        })
    }
}

impl TokenClient for GithubTokenClient {
    fn exchange(&self, fields: &[(&str, &str)]) -> Result<AccessTokenResponse, GithubAuthFailure> {
        let client = build_client()?;
        let response = post_form(&client, &self.url, fields)?;
        let (status, body) = read_body(response)?;
        // An outage is an outage whatever its body holds. GitHub's edge
        // answers a rate limit with a page, not a token document, so the
        // status is read before the body.
        if is_outage(status) {
            return Err(GithubAuthFailure::Offline);
        }
        let parsed = parse_body(&body)?;
        classify_token_exchange(status, parsed)
    }
}

/// Separates a successful exchange, a refused refresh grant, a client
/// misconfiguration, an outage and anything else from one token response.
///
/// A successful answer is returned with its `error` field cleared: the field
/// is body text that can echo a token, and nothing reads it once a token is
/// present.
///
/// # Errors
///
/// Returns [`GithubAuthFailure::SignInExpired`] for `bad_refresh_token` and
/// `invalid_grant`, [`GithubAuthFailure::NotConfigured`] for
/// `incorrect_client_credentials` and `unauthorized_client`,
/// [`GithubAuthFailure::Offline`] for a rate limit or a server fault, and
/// [`GithubAuthFailure::Failed`] otherwise.
pub fn classify_token_exchange(
    status: u16,
    mut parsed: AccessTokenResponse,
) -> Result<AccessTokenResponse, GithubAuthFailure> {
    if parsed
        .access_token
        .as_deref()
        .is_some_and(|token| !token.trim().is_empty())
    {
        parsed.error = None;
        return Ok(parsed);
    }
    if matches!(
        parsed.error.as_deref(),
        Some("bad_refresh_token" | "invalid_grant")
    ) {
        return Err(GithubAuthFailure::SignInExpired);
    }
    if matches!(
        parsed.error.as_deref(),
        Some("incorrect_client_credentials" | "unauthorized_client")
    ) {
        return Err(GithubAuthFailure::NotConfigured);
    }
    if status == 429 || (500..600).contains(&status) {
        return Err(GithubAuthFailure::Offline);
    }
    Err(GithubAuthFailure::Failed)
}

/// Runs GitHub's device flow to completion.
///
/// The application displays `user_code` and opens the verification page; the
/// mechanism never opens a browser and authors no sentence. The flow polls
/// every [`DEFAULT_POLL_INTERVAL`] unless the server suggests another interval
/// (never less than the default), lengthens the interval on `slow_down`, and
/// stops at the shorter of the server's `expires_in` and [`MAX_DEVICE_WAIT`].
/// `cancel` is read before every poll and every 100 ms during a wait.
///
/// # Errors
///
/// Returns [`GithubAuthFailure::Denied`] when the operator declines,
/// [`GithubAuthFailure::TimedOut`] when the wait elapses,
/// [`GithubAuthFailure::Cancelled`] when `cancel` is set,
/// [`GithubAuthFailure::Offline`] for a rate limit or a server fault, and
/// [`GithubAuthFailure::Failed`] when GitHub's answer cannot be used.
pub fn run_device_flow<F>(
    client_id: &str,
    endpoints: &DeviceFlowEndpoints<'_>,
    cancel: &AtomicBool,
    progress: &mut F,
) -> Result<TokenGrant, GithubAuthFailure>
where
    F: FnMut(GithubAuthProgress) + ?Sized,
{
    if client_id.trim().is_empty() {
        return Err(GithubAuthFailure::NotConfigured);
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(GithubAuthFailure::Cancelled);
    }
    let client = build_client()?;
    let response = post_form(
        &client,
        endpoints.device_code_url,
        &[("client_id", client_id)],
    )?;
    let (status, body) = read_body(response)?;
    if is_outage(status) {
        return Err(GithubAuthFailure::Offline);
    }
    if !(200..300).contains(&status) {
        return Err(GithubAuthFailure::Failed);
    }
    let device: DeviceCodeResponse = parse_body(&body)?;
    if device.device_code.trim().is_empty() || device.user_code.trim().is_empty() {
        return Err(GithubAuthFailure::Failed);
    }
    let verification_uri = device
        .verification_uri
        .filter(|uri| uri == GITHUB_DEVICE_VERIFICATION_URL)
        .unwrap_or_else(|| GITHUB_DEVICE_VERIFICATION_URL.to_owned());
    progress(GithubAuthProgress::AwaitingVerification {
        user_code: device.user_code.clone(),
        verification_uri: verification_uri.clone(),
    });

    let deadline = Instant::now() + Duration::from_secs(device.expires_in).min(MAX_DEVICE_WAIT);
    let mut interval =
        Duration::from_secs(device.interval.unwrap_or(DEFAULT_POLL_INTERVAL.as_secs()))
            .max(DEFAULT_POLL_INTERVAL);
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(GithubAuthFailure::Cancelled);
        }
        wait_with_cancel(interval, deadline, cancel)?;
        if cancel.load(Ordering::SeqCst) {
            return Err(GithubAuthFailure::Cancelled);
        }
        // The wait ends at the deadline, and a poll past it would answer after
        // the operator's window closed.
        if Instant::now() >= deadline {
            return Err(GithubAuthFailure::TimedOut);
        }
        let response = post_form(
            &client,
            endpoints.access_token_url,
            &[
                ("client_id", client_id),
                ("device_code", device.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ],
        )?;
        let (status, body) = read_body(response)?;
        if is_outage(status) {
            return Err(GithubAuthFailure::Offline);
        }
        let parsed: AccessTokenResponse = parse_body(&body)?;
        if parsed
            .access_token
            .as_deref()
            .is_some_and(|token| !token.trim().is_empty())
        {
            return grant_from_token_response(&parsed, SystemTime::now());
        }
        classify_device_error(parsed.error.as_deref(), parsed.interval, &mut interval)?;
        progress(GithubAuthProgress::Polling {
            user_code: device.user_code.clone(),
            verification_uri: verification_uri.clone(),
        });
    }
}

/// The stored grant, or `None` when the slot is empty.
///
/// A value that is not a recognised grant document is read as a bare access
/// token with no expiry and no refresh token: that is the shape an earlier
/// build of the application wrote, and refusing it would strand a signed-in
/// operator. Any other unreadable document is therefore not trusted as a
/// grant, and a forced refresh on it reports
/// [`GithubAuthFailure::SignInExpired`].
///
/// # Errors
///
/// Returns [`GithubAuthFailure::CredentialStoreUnavailable`] when the store
/// could not be read. A store failure is never read as "not signed in".
pub fn read_grant(scope: &CredentialScope<'_>) -> Result<Option<TokenGrant>, GithubAuthFailure> {
    match scope.store.retrieve(scope.slot) {
        Ok(Some(raw)) if !raw.trim().is_empty() => Ok(Some(parse_grant(&raw))),
        Ok(Some(_)) | Ok(None) => Ok(None),
        Err(_) => Err(GithubAuthFailure::CredentialStoreUnavailable),
    }
}

/// Reads one slot's presence and never its value.
#[must_use]
pub fn read_credential_presence(scope: &CredentialScope<'_>) -> CredentialStoreReading {
    match scope.store.retrieve(scope.slot) {
        Ok(Some(raw)) => {
            if raw.trim().is_empty() {
                CredentialStoreReading::Missing
            } else {
                CredentialStoreReading::Present
            }
        }
        Ok(None) => CredentialStoreReading::Missing,
        Err(CredentialError::Unavailable { detail }) => CredentialStoreReading::Unavailable(detail),
    }
}

/// Writes one grant to the caller's slot, replacing any stored value.
///
/// # Errors
///
/// Returns [`GithubAuthFailure::CredentialStoreUnavailable`] when the store
/// refused the write, and [`GithubAuthFailure::Failed`] when the grant could
/// not be serialized.
pub fn write_grant(
    scope: &CredentialScope<'_>,
    grant: &TokenGrant,
) -> Result<(), GithubAuthFailure> {
    let serialized = serialize_grant(grant)?;
    scope
        .store
        .store(scope.slot, serialized.expose_secret())
        .map_err(|_| GithubAuthFailure::CredentialStoreUnavailable)
}

/// Removes the grant from the caller's slot. Removing an empty slot succeeds.
///
/// A refresh never writes a removed grant back: see
/// [`fresh_access_token`], which re-reads the slot before it writes.
///
/// # Errors
///
/// Returns [`GithubAuthFailure::CredentialStoreUnavailable`] when the store
/// refused the removal.
pub fn sign_out(scope: &CredentialScope<'_>) -> Result<(), GithubAuthFailure> {
    scope
        .store
        .remove(scope.slot)
        .map_err(|_| GithubAuthFailure::CredentialStoreUnavailable)
}

/// The access token for the stored grant, refreshed first when it is inside
/// [`ACCESS_REFRESH_SKEW`] or when `force` is set.
///
/// One refresh lock covers the whole read-refresh-write sequence, so two
/// callers that need a refresh at once cause one exchange and the second
/// adopts the rotated grant. `rejected_access` names an access token the remote
/// refused: when the stored grant already holds another value, that rotated
/// grant is reused instead of spending the refresh token again.
///
/// A refresh writes only over the grant it read. The exchange takes time and
/// the store has no conditional write, so the slot can be emptied by a
/// [`sign_out`] or replaced by a new sign-in while it runs: the slot is read
/// again and, when it no longer holds this grant, the write is abandoned. A
/// sign-out therefore stays signed out (`Ok(None)`), and a newer sign-in is
/// adopted (`Ok(Some(..))` of its token) instead of being overwritten.
///
/// The call does no waiting of its own beyond its token request's
/// [`TOKEN_REQUEST_TIMEOUT`], so a caller inside
/// [`GitPolicy::refresh_credential`](crate::GitPolicy::refresh_credential) keeps
/// [`Git::run`](crate::Git::run)'s deadline rules.
///
/// # Errors
///
/// Returns [`GithubAuthFailure::SignInExpired`] when the refresh token is
/// expired or missing on an expired access token (the grant is left in place),
/// [`GithubAuthFailure::CredentialStoreUnavailable`] on a store failure, and
/// the token client's own failure otherwise.
pub fn fresh_access_token(
    scope: &CredentialScope<'_>,
    client_id: &str,
    client: &dyn TokenClient,
    now: SystemTime,
    force: bool,
    rejected_access: Option<&SecretString>,
) -> Result<Option<SecretString>, GithubAuthFailure> {
    let _guard = REFRESH_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(grant) = read_grant(scope)? else {
        return Ok(None);
    };
    if let Some(rejected) = rejected_access {
        if grant.access_token.expose_secret() != rejected.expose_secret() {
            return Ok(Some(grant.access_token));
        }
    } else if !force && access_still_valid(&grant, now) {
        return Ok(Some(grant.access_token));
    }
    if !refresh_token_usable(&grant, now) {
        if force || grant.access_expires_at.is_some() {
            return Err(GithubAuthFailure::SignInExpired);
        }
        return Ok(Some(grant.access_token));
    }
    let Some(refresh_token) = grant.refresh_token.as_ref() else {
        return Err(GithubAuthFailure::SignInExpired);
    };
    let response = client.exchange(&[
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.expose_secret()),
    ])?;
    let refreshed = grant_from_token_response(&response, now)?;
    if refreshed.refresh_token.is_none() {
        return Err(GithubAuthFailure::Failed);
    }
    // The exchange takes time, and the store has no conditional write, so a
    // sign-out or a new sign-in can land while it runs. Re-read the slot and
    // write only over the grant this refresh started from: a signed-out grant
    // is never resurrected, and a newer sign-in is adopted instead of being
    // overwritten by a stale refresh.
    match read_grant(scope)? {
        None => Ok(None),
        Some(current)
            if current.access_token.expose_secret() != grant.access_token.expose_secret() =>
        {
            Ok(Some(current.access_token))
        }
        Some(_) => {
            write_grant(scope, &refreshed)?;
            Ok(Some(refreshed.access_token))
        }
    }
}

/// Forces a refresh after a remote rejected `rejected_access`.
///
/// The stored grant is read under the same lock as
/// [`fresh_access_token`], so a concurrent refresh is shared: when the stored
/// access token already differs from the rejected one, that rotated grant is
/// returned without a second exchange.
///
/// # Errors
///
/// The same classes as [`fresh_access_token`], including
/// [`GithubAuthFailure::SignInExpired`] for a refused refresh token, which
/// leaves the stored grant in place.
pub fn force_refresh_token(
    scope: &CredentialScope<'_>,
    client_id: &str,
    client: &dyn TokenClient,
    rejected_access: &SecretString,
) -> Result<Option<SecretString>, GithubAuthFailure> {
    fresh_access_token(
        scope,
        client_id,
        client,
        SystemTime::now(),
        true,
        Some(rejected_access),
    )
}

/// The device-code response.
#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    expires_in: u64,
    #[serde(default)]
    interval: Option<u64>,
    #[serde(default)]
    verification_uri: Option<String>,
}

/// The stored document. Its fields are the grant's wire format.
#[derive(Deserialize, Serialize)]
struct StoredGrant {
    v: u8,
    access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    access_expires_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    refresh_expires_at: Option<u64>,
}

fn unix_secs(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

/// One stored expiry, when the clock can represent it.
///
/// A store that hands back an absurd number of seconds is not trusted to be a
/// time: the grant is read as having no declared expiry, which already means
/// "does not expire on its own". Nothing panics on a stored value.
fn system_time_from_unix(secs: u64) -> Option<SystemTime> {
    UNIX_EPOCH.checked_add(Duration::from_secs(secs))
}

/// `now` plus an endpoint-declared number of seconds, when the clock can
/// represent it. An expiry too far away to represent is no declared expiry,
/// which the grant already covers.
fn expiry_after(now: SystemTime, seconds: u64) -> Option<SystemTime> {
    now.checked_add(Duration::from_secs(seconds))
}

fn parse_grant(raw: &str) -> TokenGrant {
    let trimmed = raw.trim();
    if trimmed.starts_with('{')
        && let Ok(stored) = serde_json::from_str::<StoredGrant>(trimmed)
        && stored.v == GRANT_VERSION
        && !stored.access_token.trim().is_empty()
    {
        return TokenGrant {
            access_token: SecretString::from(stored.access_token),
            access_expires_at: stored.access_expires_at.and_then(system_time_from_unix),
            refresh_token: stored
                .refresh_token
                .filter(|token| !token.trim().is_empty())
                .map(SecretString::from),
            refresh_expires_at: stored.refresh_expires_at.and_then(system_time_from_unix),
        };
    }
    TokenGrant {
        access_token: SecretString::from(trimmed),
        access_expires_at: None,
        refresh_token: None,
        refresh_expires_at: None,
    }
}

fn serialize_grant(grant: &TokenGrant) -> Result<SecretString, GithubAuthFailure> {
    let stored = StoredGrant {
        v: GRANT_VERSION,
        access_token: grant.access_token.expose_secret().to_owned(),
        access_expires_at: grant.access_expires_at.and_then(unix_secs),
        refresh_token: grant
            .refresh_token
            .as_ref()
            .map(|token| token.expose_secret().to_owned()),
        refresh_expires_at: grant.refresh_expires_at.and_then(unix_secs),
    };
    serde_json::to_string(&stored)
        .map(SecretString::from)
        .map_err(|_| GithubAuthFailure::Failed)
}

fn grant_from_token_response(
    response: &AccessTokenResponse,
    now: SystemTime,
) -> Result<TokenGrant, GithubAuthFailure> {
    let access_token = response
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or(GithubAuthFailure::Failed)?;
    Ok(TokenGrant {
        access_token: SecretString::from(access_token),
        access_expires_at: response
            .expires_in
            .and_then(|seconds| expiry_after(now, seconds)),
        refresh_token: response
            .refresh_token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(SecretString::from),
        refresh_expires_at: response
            .refresh_token_expires_in
            .and_then(|seconds| expiry_after(now, seconds)),
    })
}

fn access_still_valid(grant: &TokenGrant, now: SystemTime) -> bool {
    match grant.access_expires_at {
        Some(expiry) => expiry > now + ACCESS_REFRESH_SKEW,
        None => true,
    }
}

fn refresh_token_usable(grant: &TokenGrant, now: SystemTime) -> bool {
    let Some(token) = grant.refresh_token.as_ref() else {
        return false;
    };
    if token.expose_secret().trim().is_empty() {
        return false;
    }
    match grant.refresh_expires_at {
        Some(expiry) => expiry > now,
        None => true,
    }
}

fn classify_device_error(
    error: Option<&str>,
    suggested_interval: Option<u64>,
    interval: &mut Duration,
) -> Result<(), GithubAuthFailure> {
    match error {
        Some("authorization_pending") => Ok(()),
        Some("slow_down") => {
            *interval = suggested_interval
                .map_or(interval.saturating_add(SLOW_DOWN_STEP), Duration::from_secs)
                .max(DEFAULT_POLL_INTERVAL);
            Ok(())
        }
        Some("access_denied") => Err(GithubAuthFailure::Denied),
        Some("expired_token" | "token_expired") => Err(GithubAuthFailure::TimedOut),
        _ => Err(GithubAuthFailure::Failed),
    }
}

/// Waits one poll interval, the flow's deadline, or the caller's cancellation,
/// whichever comes first.
fn wait_with_cancel(
    interval: Duration,
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<(), GithubAuthFailure> {
    // An interval too large to represent waits only until the flow's deadline,
    // so an endpoint's suggestion cannot panic the wait.
    let end = Instant::now()
        .checked_add(interval)
        .map_or(deadline, |end| end.min(deadline));
    while Instant::now() < end {
        if cancel.load(Ordering::SeqCst) {
            return Err(GithubAuthFailure::Cancelled);
        }
        thread::sleep(CANCEL_POLL_STEP);
    }
    Ok(())
}

fn form_body(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{}={}", form_encode(key), form_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn form_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            b' ' => encoded.push('+'),
            byte => {
                encoded.push('%');
                encoded.push(char::from(HEX[(byte >> 4) as usize]));
                encoded.push(char::from(HEX[(byte & 0x0F) as usize]));
            }
        }
    }
    encoded
}

pub(crate) fn build_client() -> Result<Client, GithubAuthFailure> {
    Client::builder()
        .timeout(TOKEN_REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| GithubAuthFailure::Failed)
}

fn post_form(
    client: &Client,
    url: &str,
    fields: &[(&str, &str)],
) -> Result<Response, GithubAuthFailure> {
    client
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body(fields))
        .send()
        .map_err(map_request_error)
}

pub(crate) fn read_body(response: Response) -> Result<(u16, Vec<u8>), GithubAuthFailure> {
    let status = response.status().as_u16();
    let mut body = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES)
        .read_to_end(&mut body)
        .map_err(read_error)?;
    Ok((status, body))
}

/// A body that could not be read: the request's own wall clock may have
/// expired between the headers and the body.
///
/// `reqwest` reports that case as an opaque I/O error wrapping its own timeout,
/// so the wrapped value is read before the kind is trusted.
fn read_error(error: std::io::Error) -> GithubAuthFailure {
    let timed_out = error.kind() == std::io::ErrorKind::TimedOut
        || error
            .get_ref()
            .and_then(|source| source.downcast_ref::<reqwest::Error>())
            .is_some_and(reqwest::Error::is_timeout);
    if timed_out {
        GithubAuthFailure::TimedOut
    } else {
        GithubAuthFailure::Offline
    }
}

fn parse_body<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, GithubAuthFailure> {
    serde_json::from_slice(body).map_err(|_| GithubAuthFailure::Failed)
}

fn is_outage(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

pub(crate) fn map_request_error(error: reqwest::Error) -> GithubAuthFailure {
    if error.is_timeout() {
        GithubAuthFailure::TimedOut
    } else {
        GithubAuthFailure::Offline
    }
}

/// Accepts one `https` endpoint URL.
pub(crate) fn require_https(url: &str) -> Result<(), GithubAuthFailure> {
    let parsed = reqwest::Url::parse(url).map_err(|_| GithubAuthFailure::NotConfigured)?;
    if parsed.scheme() != "https" {
        return Err(GithubAuthFailure::NotConfigured);
    }
    Ok(())
}

/// Accepts one loopback test endpoint URL: plain or TLS `http` on `127.0.0.1`
/// with an explicit port.
#[cfg(feature = "fixture-server")]
pub(crate) fn require_loopback(url: &str) -> Result<(), GithubAuthFailure> {
    let parsed = reqwest::Url::parse(url).map_err(|_| GithubAuthFailure::NotConfigured)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str() != Some("127.0.0.1")
        || parsed.port().is_none()
    {
        return Err(GithubAuthFailure::NotConfigured);
    }
    Ok(())
}
