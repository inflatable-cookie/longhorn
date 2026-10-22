//! Contract 027 (`longhorn-tool-provision`): a full live install against the
//! loopback fixture server, then an upgrade and a forced rollback.
//!
//! The artifact is a tiny `tar.gz` holding a shell script that prints its own
//! version. The version probe runs through the crate's production
//! `CommandRunner`, so the pipeline starts a real child process; the release
//! source is the crate's production `HttpReleases` pointed at the loopback
//! fixture server, so the resolve, the listing, the bounded download and the
//! checksum-file download are real HTTP. No request leaves the host.

use std::fs;
use std::path::Path;
use std::time::Duration;

use flate2::Compression;
use flate2::write::GzEncoder;
use longhorn_redact::{RedactionRules, Redactor};
use longhorn_tool_provision::{
    ArtifactShape, CommandRunner, DigestDeclaration, FixtureRoute, FixtureServer, HttpReleases,
    InstallError, InstallFault, InstallStage, ProvisionError, ReleaseOptions, ReleaseQuery,
    ReleaseRefusal, ReleaseSource, SignatureReport, SignatureVerifier, ToolDescription, ToolHealth,
    Version, install_tool, tool_health,
};
use secrecy::SecretString;
use sha2::{Digest, Sha256};

const OWNER: &str = "example-owner";
const REPOSITORY: &str = "example-tool";
const NAME: &str = "example-tool";
const BINARY: &str = "example-tool";
const ASSET: &str = "example-tool.tar.gz";
const CHECKSUMS: &str = "checksums.txt";
const VERSION_ARGUMENTS: &[&str] = &["--version"];

fn version(major: u64) -> Version {
    Version::new(major, 0, 0)
}

fn script(version: &str) -> Vec<u8> {
    format!("#!/bin/sh\nprintf 'example-tool {version}\\n'\n").into_bytes()
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// One gzipped tarball holding the executable script.
fn archive(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = tar::Builder::new(&mut encoder);
        let mut header = tar::Header::new_gnu();
        header
            .set_path(BINARY)
            .expect("the member path is a plain name");
        header.set_size(body.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append(&header, body)
            .expect("the member is appended");
        builder.finish().expect("the archive finishes");
    }
    encoder.finish().expect("the gzip stream finishes")
}

/// The release listing the production resolver reads.
fn listing(tag: &str, assets: &[&str]) -> Vec<u8> {
    let assets = assets
        .iter()
        .map(|asset| format!("{{\"name\":\"{asset}\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"tag_name\":\"{tag}\",\"name\":\"Example Tool {tag}\",\"draft\":false,\"prerelease\":false,\"assets\":[{assets}]}}"
    )
    .into_bytes()
}

/// A release served at the production URL shapes: the latest listing, the
/// pinned listing, and each asset.
fn fixture(tag: &str, assets: &[(&str, Vec<u8>)]) -> FixtureServer {
    let names = assets.iter().map(|(name, _)| *name).collect::<Vec<&str>>();
    let mut routes = vec![
        FixtureRoute::new(
            FixtureServer::latest_path(OWNER, REPOSITORY),
            200,
            listing(tag, &names),
        ),
        FixtureRoute::new(
            FixtureServer::tag_path(OWNER, REPOSITORY, tag),
            200,
            listing(tag, &names),
        ),
    ];
    for (asset, body) in assets {
        routes.push(FixtureRoute::new(
            FixtureServer::asset_path(OWNER, REPOSITORY, tag, asset),
            200,
            body.clone(),
        ));
    }
    FixtureServer::spawn(routes).expect("the loopback fixture server starts")
}

fn source(server: &FixtureServer, bound: Option<u64>) -> HttpReleases {
    let mut options = ReleaseOptions::new("longhorn-desktop-support-conformance/0.0.0")
        .with_timeout(Duration::from_secs(10));
    if let Some(max_artifact_bytes) = bound {
        options = options.with_max_artifact_bytes(max_artifact_bytes);
    }
    HttpReleases::new(
        server.endpoints().expect("loopback endpoints"),
        options,
        None::<SecretString>,
    )
    .expect("the release source builds")
}

fn redactor() -> Redactor {
    Redactor::new(Vec::<String>::new(), RedactionRules::common())
}

/// A verifier that must never run: no vendor team is declared, so the pipeline
/// has no signature to check.
struct NoSignatureVerifier;

impl SignatureVerifier for NoSignatureVerifier {
    fn report(&self, _binary: &Path) -> Result<SignatureReport, ProvisionError> {
        panic!("no vendor team is declared, so the pipeline must not ask for a signature")
    }
}

/// Directory entries of the managed `bin` directory, sorted.
fn bin_entries(root: &Path) -> Vec<String> {
    let mut names = fs::read_dir(root.join("bin"))
        .expect("the managed bin directory exists")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

pub(crate) fn run() -> String {
    let root = tempfile::tempdir().expect("a managed root");
    let first = archive(&script("1.0.0"));
    let floor = version(1);

    // A bounded download refuses the artifact before any file is created,
    // whatever the declared length says.
    let bound_server = fixture(
        "v1.0.0",
        &[
            (ASSET, first.clone()),
            (
                CHECKSUMS,
                format!("{}  {ASSET}\n", digest(&first)).into_bytes(),
            ),
        ],
    );
    let bound_source = source(&bound_server, Some(16));
    let release = bound_source
        .resolve_release(&ReleaseQuery {
            owner: OWNER,
            repository: REPOSITORY,
            pinned_tag: None,
            version_floor: Some(&floor),
        })
        .expect("the loopback listing resolves");
    assert_eq!(release.tag(), "v1.0.0");
    assert_eq!(release.version(), Some(&floor));
    assert!(release.has_asset(ASSET), "the release lists the artifact");
    let downloads = tempfile::tempdir().expect("a download directory");
    let refused = bound_source
        .download_asset(&release, ASSET, downloads.path())
        .expect_err("the artifact is over the bound");
    assert!(
        matches!(
            refused,
            longhorn_tool_provision::ReleaseError::Refused(ReleaseRefusal::ArtifactTooLarge { .. })
        ),
        "the bound refuses the download: {refused:?}"
    );
    assert_eq!(
        fs::read_dir(downloads.path())
            .expect("the download directory")
            .count(),
        0,
        "a refused download leaves no file"
    );

    // A full install: unpinned resolve, checksum file, digest, tar.gz
    // extraction, staged probe, swap, receipt, health.
    let server = fixture(
        "v1.0.0",
        &[
            (ASSET, first.clone()),
            (
                CHECKSUMS,
                format!("{}  {ASSET}\n", digest(&first)).into_bytes(),
            ),
        ],
    );
    let release_source = source(&server, None);
    let tool = ToolDescription {
        name: NAME,
        binary_name: BINARY,
        release: ReleaseQuery {
            owner: OWNER,
            repository: REPOSITORY,
            pinned_tag: None,
            version_floor: Some(&floor),
        },
        asset: ASSET,
        shape: ArtifactShape::TarGz { member: BINARY },
        digest: DigestDeclaration::ReleaseChecksums { asset: CHECKSUMS },
        vendor_team: None,
        version_arguments: VERSION_ARGUMENTS,
    };
    let mut stages = Vec::new();
    let receipt = install_tool(
        &tool,
        root.path(),
        &release_source,
        &NoSignatureVerifier,
        &CommandRunner,
        &redactor(),
        &mut |stage| stages.push(stage),
    )
    .expect("the verified artifact installs");
    assert_eq!(
        stages,
        vec![
            InstallStage::Checking,
            InstallStage::Resolving,
            InstallStage::Downloading,
            InstallStage::DownloadingChecksums,
            InstallStage::Verifying,
            InstallStage::Extracting,
            InstallStage::Probing,
            InstallStage::Installing,
        ],
        "the pipeline reports its stages in order"
    );
    assert_eq!(receipt.name(), NAME);
    assert_eq!(receipt.version(), "1.0.0");
    assert_eq!(receipt.release_tag(), Some("v1.0.0"));
    assert_eq!(receipt.replaced_version(), None);
    assert!(!receipt.already_current());

    let binary = root.path().join("bin").join(BINARY);
    assert_eq!(
        fs::read(&binary).expect("the installed binary"),
        script("1.0.0"),
        "the extracted member is the installed binary"
    );
    assert_eq!(bin_entries(root.path()), vec![BINARY.to_owned()]);

    let receipt_path = root.path().join("state/tool-provision/example-tool.json");
    let stored: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt_path).expect("the receipt"))
            .expect("receipt JSON");
    assert_eq!(stored["name"], NAME);
    assert_eq!(stored["version"], "1.0.0");
    assert_eq!(stored["release_tag"], "v1.0.0");
    assert_eq!(stored["asset"], ASSET);
    assert_eq!(
        stored["sha256"],
        digest(&script("1.0.0")),
        "the receipt records the installed binary's digest"
    );
    assert!(
        stored["installed_at_unix"]
            .as_u64()
            .is_some_and(|at| at > 0),
        "the receipt carries a unix timestamp"
    );

    match tool_health(&tool, root.path(), &CommandRunner, &redactor()) {
        ToolHealth::Installed {
            version,
            receipt_version,
            at_or_above_floor,
        } => {
            assert_eq!(version, "1.0.0", "health probes the managed binary");
            assert_eq!(receipt_version.as_deref(), Some("1.0.0"));
            assert!(at_or_above_floor, "health applies the caller's floor");
        }
        other => panic!("health after install: {other:?}"),
    }

    // An upgrade to a pinned release with a pinned digest replaces the binary
    // and the receipt, and keeps no backup behind.
    let second = archive(&script("2.0.0"));
    let second_server = fixture("v2.0.0", &[(ASSET, second.clone())]);
    let second_source = source(&second_server, None);
    let second_digest = digest(&second);
    let upgrade = ToolDescription {
        name: NAME,
        binary_name: BINARY,
        release: ReleaseQuery {
            owner: OWNER,
            repository: REPOSITORY,
            pinned_tag: Some("v2.0.0"),
            version_floor: Some(&floor),
        },
        asset: ASSET,
        shape: ArtifactShape::TarGz { member: BINARY },
        digest: DigestDeclaration::Pinned(&second_digest),
        vendor_team: None,
        version_arguments: VERSION_ARGUMENTS,
    };
    let upgraded = install_tool(
        &upgrade,
        root.path(),
        &second_source,
        &NoSignatureVerifier,
        &CommandRunner,
        &redactor(),
        &mut |_| {},
    )
    .expect("the upgrade installs");
    assert_eq!(upgraded.version(), "2.0.0");
    assert_eq!(upgraded.replaced_version(), Some("1.0.0"));
    assert_eq!(upgraded.release_tag(), Some("v2.0.0"));
    assert!(!upgraded.already_current());
    assert_eq!(
        fs::read(&binary).expect("the upgraded binary"),
        script("2.0.0"),
        "the swap replaced the binary"
    );
    assert_eq!(
        bin_entries(root.path()),
        vec![BINARY.to_owned()],
        "the previous copy is removed after the receipt commits"
    );
    let stored: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt_path).expect("the upgraded receipt"))
            .expect("receipt JSON");
    assert_eq!(stored["version"], "2.0.0");
    assert_eq!(stored["release_tag"], "v2.0.0");

    // A second run agrees with the live probe and the receipt, so it is
    // already current and downloads nothing.
    let current = install_tool(
        &upgrade,
        root.path(),
        &second_source,
        &NoSignatureVerifier,
        &CommandRunner,
        &redactor(),
        &mut |_| {},
    )
    .expect("the current install is recognised");
    assert!(current.already_current(), "the install is already current");
    assert_eq!(current.version(), "2.0.0");
    assert_eq!(current.replaced_version(), None);
    let asset_downloads = second_server
        .request_paths()
        .iter()
        .filter(|path| path.contains("/releases/download/"))
        .count();
    assert_eq!(
        asset_downloads, 1,
        "a current install downloads no artifact"
    );

    // A forced rollback: the swap happens, the atomic receipt commit cannot,
    // and the previous binary is restored.
    let third = archive(&script("3.0.0"));
    let third_server = fixture("v3.0.0", &[(ASSET, third.clone())]);
    let third_source = source(&third_server, None);
    let third_digest = digest(&third);
    let blocked = ToolDescription {
        name: NAME,
        binary_name: BINARY,
        release: ReleaseQuery {
            owner: OWNER,
            repository: REPOSITORY,
            pinned_tag: Some("v3.0.0"),
            version_floor: Some(&floor),
        },
        asset: ASSET,
        shape: ArtifactShape::TarGz { member: BINARY },
        digest: DigestDeclaration::Pinned(&third_digest),
        vendor_team: None,
        version_arguments: VERSION_ARGUMENTS,
    };
    fs::remove_file(&receipt_path).expect("the current receipt is removable");
    fs::create_dir_all(&receipt_path).expect("the receipt path becomes a directory");
    let failed = install_tool(
        &blocked,
        root.path(),
        &third_source,
        &NoSignatureVerifier,
        &CommandRunner,
        &redactor(),
        &mut |_| {},
    )
    .expect_err("the obstructed receipt commit fails after the swap");
    assert!(
        matches!(failed, InstallError::Fault(InstallFault::Io { .. })),
        "the receipt commit is an I/O fault: {failed:?}"
    );
    assert!(
        third_server
            .request_paths()
            .iter()
            .any(|path| path.contains("/releases/download/")),
        "the failure came after the artifact was downloaded"
    );
    assert_eq!(
        fs::read(&binary).expect("the restored binary"),
        script("2.0.0"),
        "rollback restored the previous binary"
    );
    assert_eq!(
        bin_entries(root.path()),
        vec![BINARY.to_owned()],
        "rollback left no backup or temporary copy behind"
    );
    assert!(receipt_path.is_dir(), "the obstruction is untouched");
    match tool_health(&tool, root.path(), &CommandRunner, &redactor()) {
        ToolHealth::Installed {
            version,
            receipt_version,
            ..
        } => {
            assert_eq!(version, "2.0.0", "health probes the restored binary");
            assert_eq!(
                receipt_version, None,
                "an unreadable receipt does not fail the probe"
            );
        }
        other => panic!("health after rollback: {other:?}"),
    }
    assert!(
        root.path()
            .join("state/tool-provision/locks")
            .join(format!("{BINARY}.lock"))
            .is_file(),
        "the install lock is a stable file that stays in place"
    );

    format!(
        "live install (unpinned resolve, checksum file, tar.gz extraction, CommandRunner probe, swap, receipt, health), bound refusal, pinned upgrade, current short-circuit and a forced rollback on a failed receipt commit"
    )
}
