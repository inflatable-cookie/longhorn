//! Contract 027, part 4b: release resolution and the bounded download, through
//! the public API and the loopback fixture server only.
//!
//! The owner, repository, tag and asset names are neutral. The token is a
//! marker string the tests look for in every error and `Debug` output.

#![cfg(feature = "fixture-server")]

mod support;

use std::{path::PathBuf, time::Duration};

use longhorn_tool_provision::{
    FixtureRoute, FixtureServer, HttpReleases, Release, ReleaseEndpoints, ReleaseError,
    ReleaseFault, ReleaseOptions, ReleaseQuery, ReleaseRefusal, ReleaseSource, version_from_tag,
};
use secrecy::SecretString;
use semver::Version;
use support::{RawServer, chunked_wire, release_listing, truncated_wire};

const OWNER: &str = "example-owner";
const REPO: &str = "example-tool";
const TAG: &str = "v0.14.1";
const ASSET: &str = "example-tool-aarch64-apple-darwin";
const TOKEN: &str = "test-token-never-logged";
const BODY: &[u8] = b"example-artifact-bytes";

fn latest_path() -> String {
    FixtureServer::latest_path(OWNER, REPO)
}

fn tag_path(tag: &str) -> String {
    FixtureServer::tag_path(OWNER, REPO, tag)
}

fn asset_path(tag: &str) -> String {
    FixtureServer::asset_path(OWNER, REPO, tag, ASSET)
}

fn query<'a>(pinned_tag: Option<&'a str>, version_floor: Option<&'a Version>) -> ReleaseQuery<'a> {
    ReleaseQuery {
        owner: OWNER,
        repository: REPO,
        pinned_tag,
        version_floor,
    }
}

/// A source over `endpoints`, with the default bounds and an optional token.
fn source_with(
    endpoints: ReleaseEndpoints,
    token: Option<&str>,
    options: ReleaseOptions,
) -> HttpReleases {
    HttpReleases::new(
        endpoints,
        options,
        token.map(|token| SecretString::from(token.to_owned())),
    )
    .expect("the source builds")
}

fn source(server: &FixtureServer, token: Option<&str>) -> HttpReleases {
    source_with(
        server.endpoints().expect("loopback endpoints"),
        token,
        ReleaseOptions::new("example-app/test"),
    )
}

/// The one-release server most tests need: the listing and its asset.
fn release_server(tag: &str, assets: &[&str]) -> FixtureServer {
    FixtureServer::spawn(vec![
        FixtureRoute::new(
            latest_path(),
            200,
            release_listing(tag, false, false, assets),
        ),
        FixtureRoute::new(
            tag_path(tag),
            200,
            release_listing(tag, false, false, assets),
        ),
        FixtureRoute::new(asset_path(tag), 200, BODY.to_vec()),
    ])
    .expect("the fixture server starts")
}

fn expect_refused(result: Result<Release, ReleaseError>) -> ReleaseRefusal {
    match result {
        Err(ReleaseError::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn expect_fault(result: Result<Release, ReleaseError>) -> ReleaseFault {
    match result {
        Err(ReleaseError::Fault(fault)) => fault,
        other => panic!("expected a fault, got {other:?}"),
    }
}

fn assert_no_token(error: &ReleaseError) {
    assert!(
        !error.to_string().contains(TOKEN),
        "token in display: {error}"
    );
    assert!(
        !format!("{error:?}").contains(TOKEN),
        "token in debug: {error:?}"
    );
}

#[test]
fn an_unpinned_query_resolves_latest_and_nothing_else() {
    let server = release_server(TAG, &[ASSET]);
    let release = source(&server, None)
        .resolve_release(&query(None, None))
        .expect("resolves");
    assert_eq!(release.tag(), TAG);
    assert_eq!(release.version(), Some(&Version::new(0, 14, 1)));
    assert_eq!(release.owner(), OWNER);
    assert_eq!(release.repository(), REPO);
    assert_eq!(server.request_paths(), vec![latest_path()]);
}

#[test]
fn a_pinned_query_takes_its_exact_tag_and_never_latest() {
    // Latest names a different release. A pinned query must take the tag
    // and never ask for latest.
    let server = FixtureServer::spawn(vec![
        FixtureRoute::new(
            tag_path(TAG),
            200,
            release_listing(TAG, false, false, &[ASSET]),
        ),
        FixtureRoute::new(
            latest_path(),
            200,
            release_listing("v0.14.0", false, false, &[ASSET]),
        ),
    ])
    .expect("the fixture server starts");
    let release = source(&server, None)
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    assert_eq!(release.tag(), TAG);
    assert_eq!(server.request_paths(), vec![tag_path(TAG)]);
}

#[test]
fn a_missing_pinned_release_is_no_compatible_release_naming_the_pin() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        latest_path(),
        200,
        release_listing("v0.14.0", false, false, &[ASSET]),
    )])
    .expect("the fixture server starts");
    let error = source(&server, None)
        .resolve_release(&query(Some(TAG), None))
        .expect_err("the missing pin refuses");
    assert_eq!(
        error,
        ReleaseError::Refused(ReleaseRefusal::NoCompatibleRelease {
            floor: None,
            requested: Some(TAG.to_owned()),
        })
    );
    assert!(error.to_string().contains(TAG), "{error}");
}

#[test]
fn a_pinned_tag_answered_with_another_tag_is_a_tag_mismatch() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        tag_path(TAG),
        200,
        release_listing("v0.14.0", false, false, &[ASSET]),
    )])
    .expect("the fixture server starts");
    let refusal = expect_refused(source(&server, None).resolve_release(&query(Some(TAG), None)));
    assert_eq!(
        refusal,
        ReleaseRefusal::TagMismatch {
            requested: TAG.to_owned(),
            found: "v0.14.0".to_owned(),
        }
    );
}

#[test]
fn a_draft_or_pre_release_is_refused_on_both_routes() {
    for (draft, prerelease) in [(true, false), (false, true)] {
        let server = FixtureServer::spawn(vec![
            FixtureRoute::new(
                latest_path(),
                200,
                release_listing(TAG, draft, prerelease, &[ASSET]),
            ),
            FixtureRoute::new(
                tag_path(TAG),
                200,
                release_listing(TAG, draft, prerelease, &[ASSET]),
            ),
        ])
        .expect("the fixture server starts");
        let unpinned = expect_refused(source(&server, None).resolve_release(&query(None, None)));
        assert_eq!(
            unpinned,
            ReleaseRefusal::NoCompatibleRelease {
                floor: None,
                requested: None,
            },
            "draft={draft} prerelease={prerelease}, unpinned"
        );
        let pinned = expect_refused(source(&server, None).resolve_release(&query(Some(TAG), None)));
        assert_eq!(
            pinned,
            ReleaseRefusal::NoCompatibleRelease {
                floor: None,
                requested: Some(TAG.to_owned()),
            },
            "draft={draft} prerelease={prerelease}, pinned"
        );
    }
}

#[test]
fn a_floor_refuses_a_lower_release_and_accepts_an_equal_one() {
    let floor = Version::new(1, 0, 0);
    let lower = FixtureServer::spawn(vec![FixtureRoute::new(
        latest_path(),
        200,
        release_listing("v0.9.0", false, false, &[ASSET]),
    )])
    .expect("the fixture server starts");
    assert_eq!(
        expect_refused(source(&lower, None).resolve_release(&query(None, Some(&floor)))),
        ReleaseRefusal::NoCompatibleRelease {
            floor: Some(floor.clone()),
            requested: None,
        }
    );
    let equal = FixtureServer::spawn(vec![FixtureRoute::new(
        latest_path(),
        200,
        release_listing("v1.0.0", false, false, &[ASSET]),
    )])
    .expect("the fixture server starts");
    let release = source(&equal, None)
        .resolve_release(&query(None, Some(&floor)))
        .expect("a release at the floor resolves");
    assert_eq!(release.version(), Some(&floor));
}

#[test]
fn a_release_with_no_version_is_refused_only_under_a_floor() {
    let floor = Version::new(1, 0, 0);
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        latest_path(),
        200,
        release_listing("nightly", false, false, &[ASSET]),
    )])
    .expect("the fixture server starts");
    assert!(matches!(
        expect_refused(source(&server, None).resolve_release(&query(None, Some(&floor)))),
        ReleaseRefusal::NoCompatibleRelease { .. }
    ));
    let release = source(&server, None)
        .resolve_release(&query(None, None))
        .expect("with no floor, a tag that names no version still resolves");
    assert_eq!(release.version(), None);
}

#[test]
fn a_malformed_listing_is_refused_as_malformed() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        latest_path(),
        200,
        b"<html>not a release listing</html>".to_vec(),
    )])
    .expect("the fixture server starts");
    assert!(matches!(
        expect_refused(source(&server, None).resolve_release(&query(None, None))),
        ReleaseRefusal::Malformed { .. }
    ));
}

#[test]
fn a_tag_that_is_not_a_plain_name_is_refused_as_malformed() {
    let server = FixtureServer::spawn(vec![FixtureRoute::new(
        latest_path(),
        200,
        release_listing("v1 beta", false, false, &[ASSET]),
    )])
    .expect("the fixture server starts");
    assert!(matches!(
        expect_refused(source(&server, None).resolve_release(&query(None, None))),
        ReleaseRefusal::Malformed { .. }
    ));
}

#[test]
fn a_rate_limit_with_a_reset_names_the_reset() {
    let server = FixtureServer::spawn(vec![
        FixtureRoute::new(latest_path(), 403, b"{}".to_vec())
            .with_header("X-RateLimit-Remaining", "0")
            .with_header("X-RateLimit-Reset", "1799856000"),
    ])
    .expect("the fixture server starts");
    let fault = expect_fault(source(&server, None).resolve_release(&query(None, None)));
    assert_eq!(
        fault,
        ReleaseFault::RateLimited {
            reset_unix: Some(1_799_856_000),
        }
    );
    assert!(fault.to_string().contains("1799856000"), "{fault}");
}

#[test]
fn a_rate_limit_without_a_reset_is_still_a_rate_limit() {
    let server = FixtureServer::spawn(vec![
        FixtureRoute::new(latest_path(), 429, b"{}".to_vec())
            .with_header("X-RateLimit-Remaining", "0"),
    ])
    .expect("the fixture server starts");
    assert_eq!(
        expect_fault(source(&server, None).resolve_release(&query(None, None))),
        ReleaseFault::RateLimited { reset_unix: None }
    );
}

#[test]
fn a_rate_limited_listing_is_not_paged_around() {
    // Only the rate-limited latest route exists. A paging retry would reach
    // a missing route, so one request is the whole story.
    let server = FixtureServer::spawn(vec![
        FixtureRoute::new(latest_path(), 403, b"{}".to_vec())
            .with_header("X-RateLimit-Remaining", "0"),
    ])
    .expect("the fixture server starts");
    let _ = source(&server, None).resolve_release(&query(None, None));
    assert_eq!(server.request_paths(), vec![latest_path()]);
}

#[test]
fn the_token_rides_listing_calls_and_never_an_asset_download() {
    let server = release_server(TAG, &[ASSET]);
    let source = source(&server, Some(TOKEN));
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    assert_eq!(
        server.authorization_for(&tag_path(TAG)),
        Some(format!("Bearer {TOKEN}"))
    );
    let directory = tempfile::tempdir().expect("a fresh directory");
    source
        .download_asset(&release, ASSET, directory.path())
        .expect("downloads");
    assert_eq!(server.authorization_for(&asset_path(TAG)), None);
}

#[test]
fn a_signed_out_listing_sends_no_authorization_header() {
    let server = release_server(TAG, &[ASSET]);
    source(&server, None)
        .resolve_release(&query(None, None))
        .expect("resolves");
    assert_eq!(server.authorization_for(&latest_path()), None);
}

#[test]
fn a_listing_redirect_to_the_same_origin_keeps_the_token() {
    let canonical = "/canonical-listing";
    let server = FixtureServer::spawn(vec![
        FixtureRoute::new(tag_path(TAG), 302, Vec::new()).with_header("Location", canonical),
        FixtureRoute::new(canonical, 200, release_listing(TAG, false, false, &[ASSET])),
    ])
    .expect("the fixture server starts");
    let release = source(&server, Some(TOKEN))
        .resolve_release(&query(Some(TAG), None))
        .expect("the same-origin redirect is followed");
    assert_eq!(release.tag(), TAG);
    assert_eq!(
        server.authorization_for(canonical),
        Some(format!("Bearer {TOKEN}"))
    );
}

#[test]
fn a_listing_redirect_to_another_origin_is_refused_and_the_token_stays_home() {
    let elsewhere = FixtureServer::spawn(vec![FixtureRoute::new(
        tag_path(TAG),
        200,
        release_listing(TAG, false, false, &[ASSET]),
    )])
    .expect("the second fixture server starts");
    let server = FixtureServer::spawn(vec![
        FixtureRoute::new(tag_path(TAG), 302, Vec::new()).with_header(
            "Location",
            &format!("{}{}", elsewhere.base_url(), tag_path(TAG)),
        ),
    ])
    .expect("the fixture server starts");
    let error = source(&server, Some(TOKEN))
        .resolve_release(&query(Some(TAG), None))
        .expect_err("a cross-origin listing redirect refuses");
    assert!(
        matches!(
            &error,
            ReleaseError::Refused(ReleaseRefusal::UnsafeRedirect { .. })
        ),
        "{error:?}"
    );
    assert!(
        elsewhere.request_paths().is_empty(),
        "the token-bearing request never reached the other origin"
    );
    assert_no_token(&error);
}

#[test]
fn a_download_redirect_carries_no_token_and_the_bytes_arrive() {
    let elsewhere =
        FixtureServer::spawn(vec![FixtureRoute::new(asset_path(TAG), 200, BODY.to_vec())])
            .expect("the second fixture server starts");
    let server = FixtureServer::spawn(vec![
        FixtureRoute::new(
            tag_path(TAG),
            200,
            release_listing(TAG, false, false, &[ASSET]),
        ),
        FixtureRoute::new(asset_path(TAG), 302, Vec::new()).with_header(
            "Location",
            &format!("{}{}", elsewhere.base_url(), asset_path(TAG)),
        ),
    ])
    .expect("the fixture server starts");
    let source = source(&server, Some(TOKEN));
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let path = source
        .download_asset(&release, ASSET, directory.path())
        .expect("the download follows the https-or-loopback redirect");
    assert_eq!(std::fs::read(&path).expect("the file is readable"), BODY);
    assert_eq!(server.authorization_for(&asset_path(TAG)), None);
    assert_eq!(elsewhere.authorization_for(&asset_path(TAG)), None);
    assert_eq!(elsewhere.request_paths(), vec![asset_path(TAG)]);
}

#[test]
fn an_api_call_budget_of_two_holds_for_status_and_install() {
    for pinned in [Some(TAG), None] {
        let server = release_server(TAG, &[ASSET]);
        let source = source(&server, None);
        // The status check and the install each resolve once; the install
        // then downloads the asset off the download host.
        source
            .resolve_release(&query(pinned, None))
            .expect("status check");
        let release = source
            .resolve_release(&query(pinned, None))
            .expect("install");
        let directory = tempfile::tempdir().expect("a fresh directory");
        source
            .download_asset(&release, ASSET, directory.path())
            .expect("install downloads");
        let api_calls = server
            .request_paths()
            .into_iter()
            .filter(|path| path.starts_with("/repos/"))
            .count();
        assert_eq!(
            api_calls,
            2,
            "pinned={pinned:?}: {:?}",
            server.request_paths()
        );
    }
}

#[test]
fn a_download_writes_exactly_the_bytes_the_server_sent() {
    let server = release_server(TAG, &[ASSET]);
    let source = source(&server, None);
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let path = source
        .download_asset(&release, ASSET, directory.path())
        .expect("downloads");
    assert_eq!(path, directory.path().join(ASSET));
    assert_eq!(std::fs::read(&path).expect("the file is readable"), BODY);
}

#[test]
fn a_missing_asset_is_refused_before_any_request() {
    let server = release_server(TAG, &["some-other-asset"]);
    let source = source(&server, None);
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let error = source
        .download_asset(&release, ASSET, directory.path())
        .expect_err("a missing asset refuses");
    assert_eq!(
        error,
        ReleaseError::Refused(ReleaseRefusal::MissingAsset {
            release: TAG.to_owned(),
            asset: ASSET.to_owned(),
        })
    );
    assert!(!server.request_paths().contains(&asset_path(TAG)));
    assert!(!directory.path().join(ASSET).exists());
}

#[test]
fn a_declared_length_over_the_bound_is_refused_before_any_write() {
    let server = release_server(TAG, &[ASSET]);
    let source = source_with(
        server.endpoints().expect("loopback endpoints"),
        None,
        ReleaseOptions::new("example-app/test").with_max_artifact_bytes(8),
    );
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let error = source
        .download_asset(&release, ASSET, directory.path())
        .expect_err("the declared length refuses");
    assert_eq!(
        error,
        ReleaseError::Refused(ReleaseRefusal::ArtifactTooLarge {
            asset: ASSET.to_owned(),
            limit: 8,
        })
    );
    assert!(!directory.path().join(ASSET).exists());
}

#[test]
fn a_stream_over_the_bound_without_a_declared_length_is_refused_and_removed() {
    let raw = RawServer::answering(chunked_wire(&[7u8; 64]), false);
    let api = release_server(TAG, &[ASSET]);
    let source = source_with(
        ReleaseEndpoints::loopback_for_tests(&api.base_url(), &raw.base_url())
            .expect("loopback endpoints"),
        None,
        ReleaseOptions::new("example-app/test").with_max_artifact_bytes(16),
    );
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let error = source
        .download_asset(&release, ASSET, directory.path())
        .expect_err("the real bytes refuse");
    assert_eq!(
        error,
        ReleaseError::Refused(ReleaseRefusal::ArtifactTooLarge {
            asset: ASSET.to_owned(),
            limit: 16,
        })
    );
    assert!(
        !directory.path().join(ASSET).exists(),
        "the partial file is removed"
    );
}

#[test]
fn a_truncated_download_is_a_fault_and_leaves_no_partial_file() {
    let raw = RawServer::answering(truncated_wire(64, &[1u8; 10]), false);
    let api = release_server(TAG, &[ASSET]);
    let source = source_with(
        ReleaseEndpoints::loopback_for_tests(&api.base_url(), &raw.base_url())
            .expect("loopback endpoints"),
        None,
        ReleaseOptions::new("example-app/test"),
    );
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let fault = expect_fault_download(source.download_asset(&release, ASSET, directory.path()));
    assert!(
        matches!(fault, ReleaseFault::Unreachable { .. }),
        "{fault:?}"
    );
    assert!(
        !directory.path().join(ASSET).exists(),
        "the partial file is removed"
    );
}

#[test]
fn an_existing_file_is_an_io_fault_and_is_not_overwritten() {
    let server = release_server(TAG, &[ASSET]);
    let source = source(&server, None);
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let existing = directory.path().join(ASSET);
    std::fs::write(&existing, b"keep").expect("the fixture file writes");
    let fault = expect_fault_download(source.download_asset(&release, ASSET, directory.path()));
    assert!(matches!(fault, ReleaseFault::Io { .. }), "{fault:?}");
    assert_eq!(std::fs::read(&existing).expect("readable"), b"keep");
}

#[test]
fn a_silent_source_times_out_as_unreachable() {
    let silent = RawServer::answering(Vec::new(), true);
    let endpoints = ReleaseEndpoints::loopback_for_tests(&silent.base_url(), &silent.base_url())
        .expect("loopback endpoints");
    let source = source_with(
        endpoints,
        None,
        ReleaseOptions::new("example-app/test").with_timeout(Duration::from_millis(200)),
    );
    let fault = expect_fault(source.resolve_release(&query(None, None)));
    assert!(
        matches!(fault, ReleaseFault::Unreachable { .. }),
        "{fault:?}"
    );
}

#[test]
fn the_token_is_absent_from_every_error_and_debug_output() {
    let limited = FixtureServer::spawn(vec![
        FixtureRoute::new(latest_path(), 403, b"{}".to_vec())
            .with_header("X-RateLimit-Remaining", "0"),
    ])
    .expect("the fixture server starts");
    let malformed =
        FixtureServer::spawn(vec![FixtureRoute::new(latest_path(), 200, b"no".to_vec())])
            .expect("the fixture server starts");
    let missing = FixtureServer::spawn(Vec::new()).expect("the fixture server starts");
    let elsewhere = FixtureServer::spawn(Vec::new()).expect("the second fixture server starts");
    let redirecting = FixtureServer::spawn(vec![
        FixtureRoute::new(latest_path(), 302, Vec::new()).with_header(
            "Location",
            &format!("{}{}", elsewhere.base_url(), latest_path()),
        ),
    ])
    .expect("the fixture server starts");

    for server in [&limited, &malformed, &missing, &redirecting] {
        let error = source(server, Some(TOKEN))
            .resolve_release(&query(None, None))
            .expect_err("each scenario refuses");
        assert_no_token(&error);
    }
    let server = release_server(TAG, &["another-asset"]);
    let source = source(&server, Some(TOKEN));
    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let missing_asset = source
        .download_asset(&release, ASSET, directory.path())
        .expect_err("a missing asset refuses");
    assert_no_token(&missing_asset);
    assert!(!format!("{source:?}").contains(TOKEN), "{source:?}");
    assert!(!format!("{release:?}").contains(TOKEN), "{release:?}");
}

#[test]
fn a_non_https_endpoint_is_refused_outside_the_fixture_constructor() {
    let plain_api = ReleaseEndpoints::new("http://api.example.test", "https://github.example.test");
    assert!(matches!(
        plain_api,
        Err(ReleaseError::Fault(ReleaseFault::Usage { .. }))
    ));
    let plain_download =
        ReleaseEndpoints::new("https://api.example.test", "http://github.example.test");
    assert!(matches!(
        plain_download,
        Err(ReleaseError::Fault(ReleaseFault::Usage { .. }))
    ));
    let remote_fixture =
        ReleaseEndpoints::loopback_for_tests("http://api.example.test", "http://127.0.0.1:1");
    assert!(matches!(
        remote_fixture,
        Err(ReleaseError::Fault(ReleaseFault::Usage { .. }))
    ));
    assert!(ReleaseEndpoints::new("https://api.github.com", "https://github.com").is_ok());
}

#[test]
fn a_name_with_a_separator_is_a_usage_fault_before_any_request() {
    let server = release_server(TAG, &[ASSET]);
    let source = source(&server, None);
    let pinned = source.resolve_release(&query(Some("v1/../x"), None));
    assert!(matches!(expect_fault(pinned), ReleaseFault::Usage { .. }));
    let owner = source.resolve_release(&ReleaseQuery {
        owner: "bad/owner",
        repository: REPO,
        pinned_tag: None,
        version_floor: None,
    });
    assert!(matches!(expect_fault(owner), ReleaseFault::Usage { .. }));
    assert!(
        server.request_paths().is_empty(),
        "{:?}",
        server.request_paths()
    );

    let release = source
        .resolve_release(&query(Some(TAG), None))
        .expect("resolves");
    let directory = tempfile::tempdir().expect("a fresh directory");
    let asset = source.download_asset(&release, "../escape", directory.path());
    assert!(matches!(
        expect_fault_download(asset),
        ReleaseFault::Usage { .. }
    ));
    assert!(!directory.path().join("escape").exists());
}

#[test]
fn an_invalid_user_agent_timeout_or_bound_is_a_usage_fault() {
    let endpoints = || ReleaseEndpoints::github();
    let zero_bound = HttpReleases::new(
        endpoints(),
        ReleaseOptions::new("example-app/test").with_max_artifact_bytes(0),
        None,
    );
    assert!(matches!(
        zero_bound,
        Err(ReleaseError::Fault(ReleaseFault::Usage { .. }))
    ));
    let zero_timeout = HttpReleases::new(
        endpoints(),
        ReleaseOptions::new("example-app/test").with_timeout(Duration::ZERO),
        None,
    );
    assert!(matches!(
        zero_timeout,
        Err(ReleaseError::Fault(ReleaseFault::Usage { .. }))
    ));
    let blank_agent = HttpReleases::new(endpoints(), ReleaseOptions::new("   "), None);
    assert!(matches!(
        blank_agent,
        Err(ReleaseError::Fault(ReleaseFault::Usage { .. }))
    ));
}

#[test]
fn version_from_tag_reads_the_common_tag_shapes() {
    assert_eq!(version_from_tag("v1.2.3"), Some(Version::new(1, 2, 3)));
    assert_eq!(version_from_tag("1.2.3"), Some(Version::new(1, 2, 3)));
    assert_eq!(version_from_tag("rust-v1.2.3"), Some(Version::new(1, 2, 3)));
    assert_eq!(version_from_tag("nightly"), None);
}

/// Unwraps a download's fault, or fails the test on any other result.
fn expect_fault_download(result: Result<PathBuf, ReleaseError>) -> ReleaseFault {
    match result {
        Err(ReleaseError::Fault(fault)) => fault,
        other => panic!("expected a download fault, got {other:?}"),
    }
}
