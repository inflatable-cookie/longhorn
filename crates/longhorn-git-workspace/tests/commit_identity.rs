//! The GitHub commit identity, through the public API (contract 029, part
//! 5b).
//!
//! The profile request runs against a loopback fixture server with the shared
//! in-memory credential store. Nothing here reaches the network.

#![cfg(feature = "fixture-server")]

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, SystemTime},
};

use longhorn_core::{CredentialSlot, MemoryCredentialStore};
use longhorn_git_workspace::{
    ACCESS_REFRESH_SKEW, AccessTokenResponse, CredentialScope, FixtureRoute, FixtureServer,
    GitCommitIdentity, GithubAuthFailure, GithubCommitIdentityResolver, TokenClient, TokenGrant,
    write_grant,
};
use secrecy::SecretString;

const CLIENT_ID: &str = "test-client-id";
const USER_AGENT: &str = "longhorn-tests/1";
const USER_API_PATH: &str = "/user";

fn slot() -> CredentialSlot {
    CredentialSlot::consumer_scoped("longhorn-tests", "github-com", "device-token")
        .expect("a consumer-scoped test slot")
}

/// A token client that must not be asked to exchange anything: every grant here
/// is far enough from expiry.
struct NoExchange {
    calls: AtomicUsize,
}

impl NoExchange {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl TokenClient for NoExchange {
    fn exchange(&self, _fields: &[(&str, &str)]) -> Result<AccessTokenResponse, GithubAuthFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(GithubAuthFailure::Failed)
    }
}

fn seed(store: &MemoryCredentialStore, access: &str) {
    let grant = TokenGrant {
        access_token: SecretString::from(access),
        access_expires_at: Some(SystemTime::now() + Duration::from_secs(86_400)),
        refresh_token: Some(SecretString::from("refresh")),
        refresh_expires_at: Some(SystemTime::now() + Duration::from_secs(15_897_600)),
    };
    write_grant(
        &CredentialScope {
            store,
            slot: &slot(),
        },
        &grant,
    )
    .expect("the seeded grant");
}

fn profile_body(id: u64, login: &str, name: Option<&str>) -> String {
    serde_json::json!({ "id": id, "login": login, "name": name }).to_string()
}

fn resolver(server: &FixtureServer) -> GithubCommitIdentityResolver {
    GithubCommitIdentityResolver::loopback_for_tests(&server.url(USER_API_PATH), USER_AGENT)
        .expect("a loopback resolver")
}

#[test]
fn the_identity_is_the_accounts_noreply_address() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        USER_API_PATH,
        200,
        profile_body(424_242, "octo-user", Some("Octo User")),
    )])
    .expect("a loopback fixture server");
    let store = MemoryCredentialStore::new();
    seed(&store, "access-valid");
    let resolver = resolver(&server);
    let client = NoExchange::new();
    let scope = CredentialScope {
        store: &store,
        slot: &slot(),
    };

    let identity = resolver
        .resolve_identity(&scope, CLIENT_ID, &client, SystemTime::now())
        .expect("the profile answers");

    assert_eq!(
        identity,
        GitCommitIdentity {
            name: "Octo User".to_owned(),
            email: "424242+octo-user@users.noreply.github.com".to_owned(),
        }
    );
    assert_eq!(client.calls.load(Ordering::SeqCst), 0);
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, USER_API_PATH);
    assert_eq!(
        requests[0].header("authorization"),
        Some("Bearer access-valid")
    );
    assert_eq!(
        requests[0].header("accept"),
        Some("application/vnd.github+json")
    );
    assert_eq!(requests[0].header("user-agent"), Some(USER_AGENT));
}

#[test]
fn the_identity_cache_spares_the_request_until_the_token_or_the_cache_changes() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        USER_API_PATH,
        200,
        profile_body(424_242, "octo-user", Some("Octo User")),
    )])
    .expect("a loopback fixture server");
    let store = MemoryCredentialStore::new();
    seed(&store, "access-one");
    let resolver = resolver(&server);
    let client = NoExchange::new();
    let scope = CredentialScope {
        store: &store,
        slot: &slot(),
    };

    resolver
        .resolve_identity(&scope, CLIENT_ID, &client, SystemTime::now())
        .expect("the first profile answers");
    resolver
        .resolve_identity(&scope, CLIENT_ID, &client, SystemTime::now())
        .expect("the cached identity answers");
    assert_eq!(server.requests().len(), 1);

    // A rotated access token has its own digest, so it is looked up afresh.
    seed(&store, "access-two");
    resolver
        .resolve_identity(&scope, CLIENT_ID, &client, SystemTime::now())
        .expect("the rotated profile answers");
    assert_eq!(server.requests().len(), 2);
    assert_eq!(
        server.requests()[1].header("authorization"),
        Some("Bearer access-two")
    );

    // Sign-out clears the cache, so a new sign-in is read again.
    resolver.clear_cache();
    resolver
        .resolve_identity(&scope, CLIENT_ID, &client, SystemTime::now())
        .expect("the fresh profile answers");
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn a_blank_display_name_falls_back_to_the_login() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        USER_API_PATH,
        200,
        profile_body(7, "octo-user", Some("   ")),
    )])
    .expect("a loopback fixture server");
    let store = MemoryCredentialStore::new();
    seed(&store, "access-valid");
    let resolver = resolver(&server);
    let scope = CredentialScope {
        store: &store,
        slot: &slot(),
    };

    let identity = resolver
        .resolve_identity(&scope, CLIENT_ID, &NoExchange::new(), SystemTime::now())
        .expect("the profile answers");

    assert_eq!(identity.name, "octo-user");
}

#[test]
fn an_unusable_profile_is_a_failure_and_an_unauthorised_one_expires_the_sign_in() {
    let unusable = FixtureServer::spawn(vec![FixtureRoute::new(
        USER_API_PATH,
        200,
        profile_body(0, "octo-user", None),
    )])
    .expect("a loopback fixture server");
    let store = MemoryCredentialStore::new();
    seed(&store, "access-valid");
    let scope = CredentialScope {
        store: &store,
        slot: &slot(),
    };
    assert_eq!(
        resolver(&unusable)
            .resolve_identity(&scope, CLIENT_ID, &NoExchange::new(), SystemTime::now())
            .expect_err("an account id of zero is not a profile"),
        GithubAuthFailure::Failed
    );

    let invalid_login = FixtureServer::spawn(vec![FixtureRoute::new(
        USER_API_PATH,
        200,
        profile_body(1, "octo user", None),
    )])
    .expect("a loopback fixture server");
    assert_eq!(
        resolver(&invalid_login)
            .resolve_identity(&scope, CLIENT_ID, &NoExchange::new(), SystemTime::now())
            .expect_err("a login is not an address component"),
        GithubAuthFailure::Failed
    );

    let unauthorised = FixtureServer::spawn(vec![FixtureRoute::new(USER_API_PATH, 401, "{}")])
        .expect("a loopback fixture server");
    assert_eq!(
        resolver(&unauthorised)
            .resolve_identity(&scope, CLIENT_ID, &NoExchange::new(), SystemTime::now())
            .expect_err("the token was refused"),
        GithubAuthFailure::SignInExpired
    );

    for (status, expected) in [
        (429, GithubAuthFailure::Offline),
        (503, GithubAuthFailure::Offline),
    ] {
        let down = FixtureServer::spawn(vec![FixtureRoute::new(USER_API_PATH, status, "{}")])
            .expect("a loopback fixture server");
        assert_eq!(
            resolver(&down)
                .resolve_identity(&scope, CLIENT_ID, &NoExchange::new(), SystemTime::now())
                .expect_err("the profile endpoint is down"),
            expected
        );
    }

    let forbidden = FixtureServer::spawn(vec![FixtureRoute::new(USER_API_PATH, 403, "{}")])
        .expect("a loopback fixture server");
    assert_eq!(
        resolver(&forbidden)
            .resolve_identity(&scope, CLIENT_ID, &NoExchange::new(), SystemTime::now())
            .expect_err("a forbidden profile is not usable"),
        GithubAuthFailure::Failed
    );
}

#[test]
fn an_empty_slot_is_not_signed_in() {
    let server = FixtureServer::spawn(Vec::new()).expect("a loopback fixture server");
    let store = MemoryCredentialStore::new();
    let scope = CredentialScope {
        store: &store,
        slot: &slot(),
    };

    assert_eq!(
        resolver(&server)
            .resolve_identity(&scope, CLIENT_ID, &NoExchange::new(), SystemTime::now())
            .expect_err("nothing is stored"),
        GithubAuthFailure::NotSignedIn
    );
    assert!(server.requests().is_empty());
}

#[test]
fn an_identity_read_refreshes_a_grant_inside_the_skew() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        USER_API_PATH,
        200,
        profile_body(9, "octo-user", None),
    )])
    .expect("a loopback fixture server");
    let store = MemoryCredentialStore::new();
    let grant = TokenGrant {
        access_token: SecretString::from("access-soon"),
        access_expires_at: Some(SystemTime::now() + ACCESS_REFRESH_SKEW / 2),
        refresh_token: Some(SecretString::from("refresh-valid")),
        refresh_expires_at: Some(SystemTime::now() + Duration::from_secs(86_400)),
    };
    write_grant(
        &CredentialScope {
            store: &store,
            slot: &slot(),
        },
        &grant,
    )
    .expect("the seeded grant");
    let client = ExchangingClient;
    let scope = CredentialScope {
        store: &store,
        slot: &slot(),
    };

    resolver(&server)
        .resolve_identity(&scope, CLIENT_ID, &client, SystemTime::now())
        .expect("the refreshed profile answers");

    assert_eq!(
        server.requests()[0].header("authorization"),
        Some("Bearer access-refreshed")
    );
}

/// A token client that rotates the grant.
struct ExchangingClient;

impl TokenClient for ExchangingClient {
    fn exchange(&self, fields: &[(&str, &str)]) -> Result<AccessTokenResponse, GithubAuthFailure> {
        assert!(
            fields
                .iter()
                .any(|(key, value)| *key == "grant_type" && *value == "refresh_token")
        );
        Ok(AccessTokenResponse {
            access_token: Some("access-refreshed".to_owned()),
            expires_in: Some(28_800),
            refresh_token: Some("refresh-rotated".to_owned()),
            refresh_token_expires_in: Some(15_897_600),
            error: None,
            interval: None,
        })
    }
}

#[test]
fn plain_http_and_an_empty_user_agent_are_configuration_failures() {
    assert_eq!(
        GithubCommitIdentityResolver::new("http://127.0.0.1:9/user", USER_AGENT)
            .expect_err("plain http is not a production endpoint"),
        GithubAuthFailure::NotConfigured
    );
    assert_eq!(
        GithubCommitIdentityResolver::new("https://api.github.com/user", "  ")
            .expect_err("a user agent is required"),
        GithubAuthFailure::NotConfigured
    );
    assert!(
        GithubCommitIdentityResolver::loopback_for_tests("https://api.github.com/user", USER_AGENT)
            .is_err(),
        "the loopback constructor accepts loopback hosts only"
    );
}

#[test]
fn the_resolver_debug_names_neither_the_agent_secret_nor_a_token() {
    let server = FixtureServer::spawn(Vec::new()).expect("a loopback fixture server");
    let debug = format!("{:?}", resolver(&server));
    assert!(debug.contains(USER_API_PATH));
    assert!(!debug.contains("Bearer"));
    assert!(!debug.contains("token"));
}
