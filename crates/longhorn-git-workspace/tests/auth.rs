//! Device-flow sign-in, the stored grant and its refresh, sign-out, and the
//! credential-store readings through the public API (contract 029, part 5b).
//!
//! The device flow, the token exchange and the profile request run against a
//! loopback fixture server. The store is the shared in-memory one. Nothing here
//! reaches the network.

#![cfg(feature = "fixture-server")]

use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use longhorn_core::{CredentialSlot, CredentialStore, MemoryCredentialStore};
use longhorn_git_workspace::{
    ACCESS_REFRESH_SKEW, AccessTokenResponse, CredentialScope, CredentialStoreReading,
    DEFAULT_POLL_INTERVAL, DeviceFlowEndpoints, GITHUB_DEVICE_VERIFICATION_URL, GithubAuthFailure,
    GithubAuthProgress, GithubTokenClient, MAX_DEVICE_WAIT, TokenClient, TokenGrant,
    classify_token_exchange, force_refresh_token, fresh_access_token, read_credential_presence,
    read_grant, run_device_flow, sign_out, write_grant,
};
use longhorn_git_workspace::{FixtureRoute, FixtureServer};
use secrecy::{ExposeSecret as _, SecretString};

/// A public registration fixture: the mechanism never sees the application's
/// own client id.
const CLIENT_ID: &str = "test-client-id";

/// The device-code and token paths the fixture server answers.
const DEVICE_CODE_PATH: &str = "/login/device/code";
const ACCESS_TOKEN_PATH: &str = "/login/oauth/access_token";
const USER_API_PATH: &str = "/user";

fn test_slot() -> CredentialSlot {
    CredentialSlot::consumer_scoped("longhorn-tests", "github-com", "device-token")
        .expect("a consumer-scoped test slot")
}

fn scope<'a>(store: &'a MemoryCredentialStore, slot: &'a CredentialSlot) -> CredentialScope<'a> {
    CredentialScope { store, slot }
}

fn device_body(user_code: &str, expires_in: u64, interval: Option<u64>) -> String {
    serde_json::json!({
        "device_code": "device-code-value",
        "user_code": user_code,
        "verification_uri": GITHUB_DEVICE_VERIFICATION_URL,
        "expires_in": expires_in,
        "interval": interval,
    })
    .to_string()
}

fn token_body(access: &str, refresh: &str, access_secs: u64, refresh_secs: u64) -> String {
    serde_json::json!({
        "access_token": access,
        "token_type": "bearer",
        "expires_in": access_secs,
        "refresh_token": refresh,
        "refresh_token_expires_in": refresh_secs,
    })
    .to_string()
}

fn device_error_body(error: &str) -> String {
    serde_json::json!({ "error": error }).to_string()
}

/// A server with the device-code, token and profile routes, each answering a
/// script in order and repeating the last answer.
fn auth_server(
    device: Vec<(u16, String)>,
    token: Vec<(u16, String)>,
    profile: Vec<(u16, String)>,
) -> FixtureServer {
    FixtureServer::spawn(vec![
        FixtureRoute::script(DEVICE_CODE_PATH, device),
        FixtureRoute::script(ACCESS_TOKEN_PATH, token),
        FixtureRoute::script(USER_API_PATH, profile),
    ])
    .expect("a loopback fixture server")
}

fn run_flow(
    server: &FixtureServer,
    cancel: &AtomicBool,
) -> (
    Result<TokenGrant, GithubAuthFailure>,
    Vec<GithubAuthProgress>,
) {
    let device = server.url(DEVICE_CODE_PATH);
    let token = server.url(ACCESS_TOKEN_PATH);
    let endpoints =
        DeviceFlowEndpoints::loopback_for_tests(&device, &token).expect("loopback endpoints");
    let mut progress = Vec::new();
    let result = run_device_flow(CLIENT_ID, &endpoints, cancel, &mut |event| {
        progress.push(event);
    });
    (result, progress)
}

struct FakeTokenClient {
    responses: Mutex<VecDeque<Result<AccessTokenResponse, GithubAuthFailure>>>,
    requests: Mutex<Vec<Vec<(String, String)>>>,
    delay: Duration,
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl FakeTokenClient {
    fn new(responses: Vec<Result<AccessTokenResponse, GithubAuthFailure>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
            delay: Duration::ZERO,
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    fn request_count(&self) -> usize {
        self.requests
            .lock()
            .expect("the request log is not poisoned")
            .len()
    }

    fn recorded_grant_types(&self) -> Vec<String> {
        self.requests
            .lock()
            .expect("the request log is not poisoned")
            .iter()
            .filter_map(|fields| {
                fields
                    .iter()
                    .find(|(key, _)| key == "grant_type")
                    .map(|(_, value)| value.clone())
            })
            .collect()
    }
}

impl TokenClient for FakeTokenClient {
    fn exchange(&self, fields: &[(&str, &str)]) -> Result<AccessTokenResponse, GithubAuthFailure> {
        let current = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(current, Ordering::SeqCst);
        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }
        self.requests
            .lock()
            .expect("the request log is not poisoned")
            .push(
                fields
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect(),
            );
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.responses
            .lock()
            .expect("the scripted responses are not poisoned")
            .pop_front()
            .expect("scripted token response")
    }
}

fn token_response(access: &str, refresh: &str, access_secs: u64) -> AccessTokenResponse {
    AccessTokenResponse {
        access_token: Some(access.to_owned()),
        expires_in: Some(access_secs),
        refresh_token: Some(refresh.to_owned()),
        refresh_token_expires_in: Some(15_897_600),
        error: None,
        interval: None,
    }
}

fn token_error(error: &str) -> AccessTokenResponse {
    AccessTokenResponse {
        error: Some(error.to_owned()),
        ..AccessTokenResponse::default()
    }
}

fn offset(now: SystemTime, seconds: i64) -> SystemTime {
    if seconds >= 0 {
        now + Duration::from_secs(seconds.unsigned_abs())
    } else {
        now - Duration::from_secs(seconds.unsigned_abs())
    }
}

fn seed_grant(
    store: &MemoryCredentialStore,
    access: &str,
    access_expires_from_now: Option<i64>,
    refresh: Option<&str>,
    refresh_expires_from_now: Option<i64>,
    now: SystemTime,
) {
    let slot = test_slot();
    let grant = TokenGrant {
        access_token: SecretString::from(access),
        access_expires_at: access_expires_from_now.map(|seconds| offset(now, seconds)),
        refresh_token: refresh.map(SecretString::from),
        refresh_expires_at: refresh_expires_from_now.map(|seconds| offset(now, seconds)),
    };
    write_grant(&scope(store, &slot), &grant).expect("the seeded grant");
}

/// A grant far enough from expiry that no refresh is attempted.
fn seed_valid_grant(store: &MemoryCredentialStore, access: &str, refresh: &str) {
    seed_grant(
        store,
        access,
        Some(86_400),
        Some(refresh),
        Some(15_897_600),
        SystemTime::now(),
    );
}

#[test]
fn a_device_flow_succeeds_and_reports_only_the_code_and_verification_page() {
    let server = auth_server(
        vec![(200, device_body("ABCD-1234", 900, Some(5)))],
        vec![(
            200,
            token_body("flow-access", "flow-refresh", 28_800, 15_897_600),
        )],
        Vec::new(),
    );
    let cancel = AtomicBool::new(false);

    let (result, progress) = run_flow(&server, &cancel);

    let grant = result.expect("the operator approved the code");
    assert_eq!(grant.access_token.expose_secret(), "flow-access");
    assert_eq!(
        grant
            .refresh_token
            .as_ref()
            .map(|token| token.expose_secret().to_owned()),
        Some("flow-refresh".to_owned())
    );
    assert!(grant.access_expires_at.is_some());
    // The first poll answered, so the code is announced once and no further
    // progress event is due.
    assert_eq!(
        progress,
        vec![GithubAuthProgress::AwaitingVerification {
            user_code: "ABCD-1234".to_owned(),
            verification_uri: GITHUB_DEVICE_VERIFICATION_URL.to_owned(),
        }]
    );

    // The device request names the client id; the poll names the device code
    // and the device-code grant type. No token value travels in a request.
    let device_bodies = server.bodies_for(DEVICE_CODE_PATH);
    assert_eq!(device_bodies, vec!["client_id=test-client-id".to_owned()]);
    let poll_bodies = server.bodies_for(ACCESS_TOKEN_PATH);
    assert_eq!(poll_bodies.len(), 1, "{poll_bodies:?}");
    assert!(
        poll_bodies[0]
            .contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code")
    );
    assert!(poll_bodies[0].contains("device_code=device-code-value"));
    assert!(!poll_bodies[0].contains("flow-access"));
}

#[test]
fn a_declined_code_is_denied_rather_than_expired() {
    let server = auth_server(
        vec![(200, device_body("DENY-0001", 900, Some(5)))],
        vec![(200, device_error_body("access_denied"))],
        Vec::new(),
    );
    let cancel = AtomicBool::new(false);

    let (result, _) = run_flow(&server, &cancel);

    assert_eq!(
        result.expect_err("the operator declined"),
        GithubAuthFailure::Denied
    );
}

#[test]
fn the_device_wait_is_the_shorter_of_the_declared_expiry_and_the_maximum() {
    assert_eq!(MAX_DEVICE_WAIT, Duration::from_secs(900));
    assert_eq!(DEFAULT_POLL_INTERVAL, Duration::from_secs(5));

    // One second of declared wait, then GitHub keeps answering "pending".
    let server = auth_server(
        vec![(200, device_body("WAIT-0001", 1, Some(5)))],
        vec![(200, device_error_body("authorization_pending"))],
        Vec::new(),
    );
    let cancel = AtomicBool::new(false);
    let started = Instant::now();

    let (result, _) = run_flow(&server, &cancel);

    assert_eq!(
        result.expect_err("the wait elapsed"),
        GithubAuthFailure::TimedOut
    );
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the flow waited {:?}",
        started.elapsed()
    );
    // The declared wait ended before the first poll was due, so GitHub was
    // never asked again.
    assert_eq!(server.request_count(ACCESS_TOKEN_PATH), 0);
}

#[test]
fn a_slow_down_answer_lengthens_the_poll_instead_of_polling_again_immediately() {
    let server = auth_server(
        vec![(200, device_body("SLOW-0001", 900, Some(5)))],
        vec![
            (200, device_error_body("slow_down")),
            (
                200,
                token_body("slow-access", "slow-refresh", 28_800, 15_897_600),
            ),
        ],
        Vec::new(),
    );
    let cancel = AtomicBool::new(false);
    let started = Instant::now();

    // Cancel after one default interval plus a margin: a flow that ignored
    // `slow_down` would have polled a second time by then, and this one has
    // to wait ten seconds instead.
    let mut progress = Vec::new();
    let result = {
        let device = server.url(DEVICE_CODE_PATH);
        let token = server.url(ACCESS_TOKEN_PATH);
        let endpoints =
            DeviceFlowEndpoints::loopback_for_tests(&device, &token).expect("loopback endpoints");
        thread::scope(|threads| {
            threads.spawn(|| {
                thread::sleep(Duration::from_secs(7));
                cancel.store(true, Ordering::SeqCst);
            });
            run_device_flow(CLIENT_ID, &endpoints, &cancel, &mut |event| {
                progress.push(event);
            })
        })
    };

    assert_eq!(
        result.expect_err("the test cancelled the flow"),
        GithubAuthFailure::Cancelled
    );
    assert_eq!(
        server.request_count(ACCESS_TOKEN_PATH),
        1,
        "the poll repeated inside the lengthened interval"
    );
    assert!(started.elapsed() >= Duration::from_secs(6));
}

#[test]
fn a_cancelled_flow_asks_github_nothing() {
    let server = auth_server(
        vec![(200, device_body("CANC-0001", 900, Some(5)))],
        vec![(200, token_body("x", "y", 10, 10))],
        Vec::new(),
    );
    let cancel = AtomicBool::new(true);

    let (result, _) = run_flow(&server, &cancel);

    assert_eq!(
        result.expect_err("cancelled before it started"),
        GithubAuthFailure::Cancelled
    );
    assert_eq!(server.request_count(DEVICE_CODE_PATH), 0);
}

#[test]
fn cancellation_ends_a_poll_wait_without_waiting_for_the_interval() {
    let server = auth_server(
        vec![(200, device_body("CANC-0002", 900, Some(5)))],
        vec![(200, device_error_body("authorization_pending"))],
        Vec::new(),
    );
    let cancel = AtomicBool::new(false);
    let device = server.url(DEVICE_CODE_PATH);
    let token = server.url(ACCESS_TOKEN_PATH);
    let endpoints =
        DeviceFlowEndpoints::loopback_for_tests(&device, &token).expect("loopback endpoints");
    let started = Instant::now();
    let result = thread::scope(|threads| {
        threads.spawn(|| {
            thread::sleep(Duration::from_millis(300));
            cancel.store(true, Ordering::SeqCst);
        });
        run_device_flow(CLIENT_ID, &endpoints, &cancel, &mut |_| {})
    });

    assert_eq!(
        result.expect_err("cancelled while waiting"),
        GithubAuthFailure::Cancelled
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "cancellation waited {:?}",
        started.elapsed()
    );
    assert_eq!(server.request_count(ACCESS_TOKEN_PATH), 0);
}

#[test]
fn a_token_endpoint_outage_is_offline_and_an_unusable_answer_is_a_failure() {
    let outage = auth_server(
        vec![(200, device_body("OUTA-0001", 900, Some(5)))],
        vec![(503, String::new())],
        Vec::new(),
    );
    let cancel = AtomicBool::new(false);
    let (result, _) = run_flow(&outage, &cancel);
    assert_eq!(
        result.expect_err("the endpoint is down"),
        GithubAuthFailure::Offline
    );

    let unknown = auth_server(
        vec![(200, device_body("UNKN-0001", 900, Some(5)))],
        vec![(200, device_error_body("server_error"))],
        Vec::new(),
    );
    let (result, _) = run_flow(&unknown, &cancel);
    assert_eq!(
        result.expect_err("GitHub's answer is unusable"),
        GithubAuthFailure::Failed
    );
}

#[test]
fn an_empty_client_id_is_a_configuration_failure() {
    let server = auth_server(Vec::new(), Vec::new(), Vec::new());
    let cancel = AtomicBool::new(false);
    let device = server.url(DEVICE_CODE_PATH);
    let token = server.url(ACCESS_TOKEN_PATH);
    let endpoints =
        DeviceFlowEndpoints::loopback_for_tests(&device, &token).expect("loopback endpoints");

    let result = run_device_flow("  ", &endpoints, &cancel, &mut |_| {});

    assert_eq!(
        result.expect_err("no registration"),
        GithubAuthFailure::NotConfigured
    );
    assert!(server.requests().is_empty());
}

#[test]
fn plain_http_is_refused_outside_the_loopback_test_constructor() {
    assert_eq!(
        DeviceFlowEndpoints::new(
            "http://127.0.0.1:9/login/device/code",
            "https://github.com/login/oauth/access_token"
        )
        .expect_err("plain http is not a production endpoint"),
        GithubAuthFailure::NotConfigured
    );
    assert_eq!(
        DeviceFlowEndpoints::new(
            "https://github.com/login/device/code",
            "http://example.test/token"
        )
        .expect_err("plain http is not a production endpoint"),
        GithubAuthFailure::NotConfigured
    );
    assert_eq!(
        GithubTokenClient::new("http://127.0.0.1:9/token")
            .expect_err("plain http is not a production endpoint"),
        GithubAuthFailure::NotConfigured
    );
    assert!(GithubTokenClient::loopback_for_tests("http://127.0.0.1:9/token").is_ok());
    let defaults = DeviceFlowEndpoints::github();
    let _ = defaults;
}

#[test]
fn the_stored_grant_is_refreshed_inside_the_skew_and_holds_no_token_in_debug() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    assert_eq!(ACCESS_REFRESH_SKEW, Duration::from_secs(300));
    seed_grant(
        &store,
        "access-soon",
        Some(60),
        Some("refresh-valid"),
        Some(86_400),
        now,
    );
    let client = FakeTokenClient::new(vec![Ok(token_response(
        "access-fresh",
        "refresh-rotated",
        28_800,
    ))]);

    let token = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None)
        .expect("the refresh succeeds")
        .expect("a grant is stored");

    assert_eq!(token.expose_secret(), "access-fresh");
    assert_eq!(client.request_count(), 1);
    assert_eq!(client.recorded_grant_types(), ["refresh_token"]);
    let stored = read_grant(&scope(&store, &slot))
        .expect("the store is readable")
        .expect("a grant is stored");
    assert_eq!(stored.access_token.expose_secret(), "access-fresh");
    assert_eq!(
        stored
            .refresh_token
            .as_ref()
            .map(|token| token.expose_secret().to_owned()),
        Some("refresh-rotated".to_owned())
    );
    let debug = format!("{stored:?}");
    assert!(debug.contains("[redacted]"));
    for secret in [
        "access-fresh",
        "refresh-rotated",
        "access-soon",
        "refresh-valid",
    ] {
        assert!(!debug.contains(secret), "{debug}");
    }
}

#[test]
fn an_access_token_outside_the_skew_is_used_as_stored() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-valid",
        Some(6 * 60),
        Some("refresh-valid"),
        Some(86_400),
        now,
    );
    let client = FakeTokenClient::new(Vec::new());

    let token = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None)
        .expect("the stored grant is valid")
        .expect("a grant is stored");

    assert_eq!(token.expose_secret(), "access-valid");
    assert_eq!(client.request_count(), 0);
}

#[test]
fn an_expired_refresh_token_asks_for_a_new_sign_in_without_exchanging() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-stale",
        Some(-10),
        Some("refresh-stale"),
        Some(-10),
        now,
    );
    let client = FakeTokenClient::new(Vec::new());

    let result = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None);

    assert_eq!(
        result.expect_err("the refresh token expired"),
        GithubAuthFailure::SignInExpired
    );
    assert_eq!(client.request_count(), 0);
}

#[test]
fn a_rejected_refresh_token_asks_for_a_new_sign_in_and_keeps_the_grant() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-stale",
        Some(-10),
        Some("refresh-rejected"),
        Some(86_400),
        now,
    );
    let client = FakeTokenClient::new(vec![Err(GithubAuthFailure::SignInExpired)]);

    let result = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None);

    assert_eq!(
        result.expect_err("GitHub refused the refresh"),
        GithubAuthFailure::SignInExpired
    );
    assert_eq!(client.request_count(), 1);
    let stored = read_grant(&scope(&store, &slot))
        .expect("the store is readable")
        .expect("the grant is kept");
    assert_eq!(stored.access_token.expose_secret(), "access-stale");
    assert_eq!(
        stored
            .refresh_token
            .as_ref()
            .map(|token| token.expose_secret().to_owned()),
        Some("refresh-rejected".to_owned())
    );
}

#[test]
fn a_token_endpoint_outage_keeps_the_stored_grant() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-soon",
        Some(60),
        Some("refresh-valid"),
        Some(86_400),
        now,
    );
    let client = FakeTokenClient::new(vec![Err(GithubAuthFailure::Offline)]);

    let result = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None);

    assert_eq!(
        result.expect_err("the endpoint is down"),
        GithubAuthFailure::Offline
    );
    let stored = read_grant(&scope(&store, &slot))
        .expect("the store is readable")
        .expect("the grant is kept");
    assert_eq!(stored.access_token.expose_secret(), "access-soon");
}

#[test]
fn a_client_registration_error_does_not_ask_for_a_new_sign_in() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-stale",
        Some(-10),
        Some("refresh-valid"),
        Some(86_400),
        now,
    );
    let client = FakeTokenClient::new(vec![Err(GithubAuthFailure::NotConfigured)]);

    let result = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None);

    assert_eq!(
        result.expect_err("the registration is wrong"),
        GithubAuthFailure::NotConfigured
    );
    assert_eq!(client.request_count(), 1);
}

#[test]
fn concurrent_callers_share_one_refresh() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-soon",
        Some(60),
        Some("refresh-once"),
        Some(86_400),
        now,
    );
    let mut client = FakeTokenClient::new(vec![Ok(token_response(
        "access-shared",
        "refresh-shared",
        28_800,
    ))]);
    client.delay = Duration::from_millis(80);
    let client = client;

    thread::scope(|threads| {
        for _ in 0..2 {
            threads.spawn(|| {
                let token =
                    fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None)
                        .expect("the shared refresh succeeds")
                        .expect("a grant is stored");
                assert_eq!(token.expose_secret(), "access-shared");
            });
        }
    });

    assert_eq!(client.request_count(), 1);
    assert_eq!(client.max_in_flight.load(Ordering::SeqCst), 1);
}

#[test]
fn concurrent_forced_refreshes_reuse_an_already_rotated_grant() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-rejected",
        Some(86_400),
        Some("refresh-once"),
        Some(86_400),
        now,
    );
    let mut client = FakeTokenClient::new(vec![Ok(token_response(
        "access-rotated",
        "refresh-rotated",
        28_800,
    ))]);
    client.delay = Duration::from_millis(80);
    let client = client;

    thread::scope(|threads| {
        for _ in 0..2 {
            threads.spawn(|| {
                let token = force_refresh_token(
                    &scope(&store, &slot),
                    CLIENT_ID,
                    &client,
                    &SecretString::from("access-rejected"),
                )
                .expect("the forced refresh succeeds")
                .expect("a grant is stored");
                assert_eq!(token.expose_secret(), "access-rotated");
            });
        }
    });

    assert_eq!(client.request_count(), 1);
    assert_eq!(client.max_in_flight.load(Ordering::SeqCst), 1);
    let stored = read_grant(&scope(&store, &slot))
        .expect("the store is readable")
        .expect("a grant is stored");
    assert_eq!(stored.access_token.expose_secret(), "access-rotated");
}

#[test]
fn a_bare_stored_value_is_read_as_an_access_token_and_cannot_be_refreshed() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    store
        .store(&slot, "legacy-access-token")
        .expect("the stored value");
    let client = FakeTokenClient::new(Vec::new());
    let now = SystemTime::now();

    let token = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, false, None)
        .expect("the legacy value is used as it is")
        .expect("a grant is stored");
    assert_eq!(token.expose_secret(), "legacy-access-token");
    assert_eq!(client.request_count(), 0);

    let forced = fresh_access_token(&scope(&store, &slot), CLIENT_ID, &client, now, true, None);
    assert_eq!(
        forced.expect_err("no refresh token"),
        GithubAuthFailure::SignInExpired
    );
    assert_eq!(client.request_count(), 0);
}

#[test]
fn the_stored_document_carries_the_version_and_both_expiries() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    let now = SystemTime::now();
    seed_grant(
        &store,
        "access-doc",
        Some(3_600),
        Some("refresh-doc"),
        Some(86_400),
        now,
    );

    let raw = store
        .retrieve(&slot)
        .expect("the store is readable")
        .expect("a grant is stored");
    let document: serde_json::Value = serde_json::from_str(&raw).expect("a JSON grant document");
    assert_eq!(document["v"], 1);
    assert_eq!(document["access_token"], "access-doc");
    assert_eq!(document["refresh_token"], "refresh-doc");
    assert!(document["access_expires_at"].is_u64());
    assert!(document["refresh_expires_at"].is_u64());

    let read = read_grant(&scope(&store, &slot))
        .expect("the store is readable")
        .expect("a grant is stored");
    assert_eq!(read.access_token.expose_secret(), "access-doc");
    // The document keeps whole seconds, so the read expiry is truncated.
    let expiries = (
        read.access_expires_at
            .expect("an access expiry")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after the epoch")
            .as_secs(),
        offset(now, 3_600)
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after the epoch")
            .as_secs(),
    );
    assert_eq!(expiries.0, expiries.1);
}

#[test]
fn store_readings_name_missing_present_and_unreadable_without_a_value() {
    struct Unreadable;

    impl CredentialStore for Unreadable {
        fn store(
            &self,
            _slot: &CredentialSlot,
            _secret: &str,
        ) -> Result<(), longhorn_core::CredentialError> {
            Ok(())
        }

        fn retrieve(
            &self,
            _slot: &CredentialSlot,
        ) -> Result<Option<String>, longhorn_core::CredentialError> {
            Err(longhorn_core::CredentialError::Unavailable {
                detail: "error code -25308".to_owned(),
            })
        }

        fn remove(&self, _slot: &CredentialSlot) -> Result<(), longhorn_core::CredentialError> {
            Ok(())
        }
    }

    let slot = test_slot();
    let empty = MemoryCredentialStore::new();
    assert_eq!(
        read_credential_presence(&scope(&empty, &slot)),
        CredentialStoreReading::Missing
    );
    let filled = MemoryCredentialStore::new();
    filled
        .store(&slot, "controlled-secret")
        .expect("the stored value");
    assert_eq!(
        read_credential_presence(&scope(&filled, &slot)),
        CredentialStoreReading::Present
    );
    let unreadable = Unreadable;
    let unreadable_scope = CredentialScope {
        store: &unreadable,
        slot: &slot,
    };
    assert_eq!(
        read_credential_presence(&unreadable_scope),
        CredentialStoreReading::Unavailable("error code -25308".to_owned())
    );
    assert_eq!(
        read_grant(&unreadable_scope).expect_err("an unreadable store is a failure"),
        GithubAuthFailure::CredentialStoreUnavailable
    );
    let client = FakeTokenClient::new(Vec::new());
    assert_eq!(
        fresh_access_token(
            &unreadable_scope,
            CLIENT_ID,
            &client,
            SystemTime::now(),
            false,
            None
        )
        .expect_err("an unreadable store is a failure"),
        GithubAuthFailure::CredentialStoreUnavailable
    );
}

#[test]
fn sign_out_removes_the_grant_and_repeats_cleanly() {
    let store = MemoryCredentialStore::new();
    let slot = test_slot();
    seed_valid_grant(&store, "access", "refresh");
    assert_eq!(
        read_credential_presence(&scope(&store, &slot)),
        CredentialStoreReading::Present
    );

    sign_out(&scope(&store, &slot)).expect("the grant is removed");
    assert_eq!(
        read_credential_presence(&scope(&store, &slot)),
        CredentialStoreReading::Missing
    );
    assert!(
        read_grant(&scope(&store, &slot))
            .expect("the store is readable")
            .is_none()
    );
    sign_out(&scope(&store, &slot)).expect("removing an empty slot is quiet");
}

#[test]
fn the_token_exchange_classification_separates_refusal_from_configuration() {
    assert!(matches!(
        classify_token_exchange(200, token_error("bad_refresh_token")),
        Err(GithubAuthFailure::SignInExpired)
    ));
    assert!(matches!(
        classify_token_exchange(401, token_error("bad_refresh_token")),
        Err(GithubAuthFailure::SignInExpired)
    ));
    assert!(matches!(
        classify_token_exchange(200, token_error("invalid_grant")),
        Err(GithubAuthFailure::SignInExpired)
    ));
    assert!(matches!(
        classify_token_exchange(401, token_error("incorrect_client_credentials")),
        Err(GithubAuthFailure::NotConfigured)
    ));
    assert!(matches!(
        classify_token_exchange(200, token_error("unauthorized_client")),
        Err(GithubAuthFailure::NotConfigured)
    ));
    assert!(matches!(
        classify_token_exchange(429, token_error("")),
        Err(GithubAuthFailure::Offline)
    ));
    assert!(matches!(
        classify_token_exchange(503, token_error("")),
        Err(GithubAuthFailure::Offline)
    ));
    assert!(matches!(
        classify_token_exchange(401, token_error("")),
        Err(GithubAuthFailure::Failed)
    ));
    assert!(matches!(
        classify_token_exchange(401, AccessTokenResponse::default()),
        Err(GithubAuthFailure::Failed)
    ));
    let granted = classify_token_exchange(200, token_response("issued", "refresh", 10))
        .expect("an access token is a success");
    assert_eq!(granted.access_token.as_deref(), Some("issued"));
}
