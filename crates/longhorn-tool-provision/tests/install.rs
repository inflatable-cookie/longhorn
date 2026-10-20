//! Contract 027, part 4c: public install, health and login-PATH behavior.

#![cfg(feature = "fixture-server")]

use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use fs4::FileExt;
use longhorn_redact::{RedactionRules, Redactor};
use longhorn_tool_provision::{
    ArtifactShape, CommandRunner, DigestDeclaration, FixtureRoute, FixtureServer, HttpReleases,
    InstallError, InstallFault, InstallRefusal, InstallStage, LoginPathCache, ProbePhase,
    ProcessOutput, ProcessRunner, Release, ReleaseError, ReleaseQuery, ReleaseSource,
    SignatureReport, SignatureVerifier, ToolDescription, ToolHealth, Version,
    find_executable_on_path, install_tool, is_executable, parse_login_path_output, tool_health,
};
use secrecy::SecretString;
use semver::Version as SemverVersion;
use sha2::{Digest, Sha256};

const OWNER: &str = "example-owner";
const REPOSITORY: &str = "example-tool";
const NAME: &str = "example-tool";
const BINARY: &str = "example-tool";
const ASSET: &str = "example-tool-linux-x86_64";
const TAG: &str = "v1.2.3";
const ARGS: &[&str] = &["--version"];

fn bytes(version: &str) -> Vec<u8> {
    format!("fixture-version:{version}\n").into_bytes()
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn listing(tag: &str, assets: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "tag_name": tag,
        "draft": false,
        "prerelease": false,
        "assets": assets.iter().map(|asset| serde_json::json!({"name": asset})).collect::<Vec<_>>(),
    }))
    .expect("release listing serializes")
}

fn fixture(tag: &str, assets: &[(&str, Vec<u8>)]) -> FixtureServer {
    let mut routes = vec![
        FixtureRoute::new(
            FixtureServer::latest_path(OWNER, REPOSITORY),
            200,
            listing(
                tag,
                &assets.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            ),
        ),
        FixtureRoute::new(
            FixtureServer::tag_path(OWNER, REPOSITORY, tag),
            200,
            listing(
                tag,
                &assets.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            ),
        ),
    ];
    for (asset, body) in assets {
        routes.push(FixtureRoute::new(
            FixtureServer::asset_path(OWNER, REPOSITORY, tag, asset),
            200,
            body.clone(),
        ));
    }
    FixtureServer::spawn(routes).expect("fixture server starts")
}

fn source(server: &FixtureServer) -> HttpReleases {
    HttpReleases::new(
        server.endpoints().expect("loopback endpoints"),
        longhorn_tool_provision::ReleaseOptions::new("longhorn-tool-provision/tests")
            .with_timeout(Duration::from_secs(2)),
        None::<SecretString>,
    )
    .expect("HTTP release source builds")
}

fn description<'a>(
    pinned_tag: Option<&'a str>,
    floor: Option<&'a SemverVersion>,
    digest: DigestDeclaration<'a>,
) -> ToolDescription<'a> {
    ToolDescription {
        name: NAME,
        binary_name: BINARY,
        release: ReleaseQuery {
            owner: OWNER,
            repository: REPOSITORY,
            pinned_tag,
            version_floor: floor,
        },
        asset: ASSET,
        shape: ArtifactShape::Raw,
        digest,
        vendor_team: None,
        version_arguments: ARGS,
    }
}

fn redactor() -> Redactor {
    Redactor::new(Vec::<String>::new(), RedactionRules::common())
}

#[cfg(unix)]
#[test]
fn production_runner_executes_a_program_with_its_argument_vector() {
    let output = CommandRunner
        .run(
            Path::new("/usr/bin/printf"),
            &[OsString::from("%s"), OsString::from("fixed argument")],
        )
        .expect("fixed program starts");
    assert!(output.success());
    assert_eq!(output.stdout(), b"fixed argument");
    assert!(output.stderr().is_empty());
}

fn version_runner() -> impl ProcessRunner {
    TestRunner(|program: &Path, arguments: &[OsString]| {
        if arguments != [OsString::from("--version")] {
            return Err(longhorn_tool_provision::RunnerError::new(
                "unexpected version-probe arguments",
            ));
        }
        let contents = fs::read(program).unwrap_or_default();
        let text = String::from_utf8_lossy(&contents);
        if text.contains("probe-fail") {
            return Ok(ProcessOutput::new(
                b"probe failed".to_vec(),
                b"process failed".to_vec(),
                false,
                Some(9),
            ));
        }
        let Some(version) = text
            .lines()
            .find_map(|line| line.strip_prefix("fixture-version:"))
        else {
            return Ok(ProcessOutput::new(
                b"no version".to_vec(),
                Vec::new(),
                true,
                Some(0),
            ));
        };
        Ok(ProcessOutput::new(
            format!("example-tool v{version}\n").into_bytes(),
            Vec::new(),
            true,
            Some(0),
        ))
    })
}

struct TestRunner<F>(F);

impl<F> ProcessRunner for TestRunner<F>
where
    F: Fn(&Path, &[OsString]) -> Result<ProcessOutput, longhorn_tool_provision::RunnerError>
        + Send
        + Sync,
{
    fn run(
        &self,
        program: &Path,
        arguments: &[OsString],
    ) -> Result<ProcessOutput, longhorn_tool_provision::RunnerError> {
        (self.0)(program, arguments)
    }
}

struct TeamVerifier;

impl SignatureVerifier for TeamVerifier {
    fn report(
        &self,
        _binary: &Path,
    ) -> Result<SignatureReport, longhorn_tool_provision::ProvisionError> {
        Ok(SignatureReport::Team("EXAMPLETEAM".to_owned()))
    }
}

#[test]
fn fresh_pinned_install_checks_sidecar_signature_and_writes_receipt() {
    let binary = bytes("1.2.3");
    let sidecar = format!("{}  {ASSET}\n", digest(&binary)).into_bytes();
    let server = fixture(
        TAG,
        &[(ASSET, binary.clone()), ("example-tool.sha256", sidecar)],
    );
    let source = source(&server);
    let root = tempfile::tempdir().expect("managed root");
    let mut tool = description(
        Some(TAG),
        None,
        DigestDeclaration::Sidecar {
            asset: "example-tool.sha256",
        },
    );
    tool.vendor_team = Some("EXAMPLETEAM");
    let mut stages = Vec::new();

    let receipt = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |stage| stages.push(stage),
    )
    .expect("verified tool installs");

    assert_eq!(receipt.name(), NAME);
    assert_eq!(receipt.version(), "1.2.3");
    assert_eq!(receipt.release_tag(), Some(TAG));
    assert_eq!(receipt.replaced_version(), None);
    assert!(!receipt.already_current());
    assert!(stages.contains(&InstallStage::VerifyingSignature));
    assert_eq!(
        fs::read(root.path().join("bin").join(BINARY)).unwrap(),
        binary
    );
    let stored: serde_json::Value = serde_json::from_slice(
        &fs::read(root.path().join("state/tool-provision/example-tool.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["version"], "1.2.3");
    assert_eq!(stored["release_tag"], TAG);
    assert_eq!(stored["asset"], ASSET);
    assert_eq!(stored["sha256"], digest(&bytes("1.2.3")));
    assert_eq!(fs::read_dir(root.path().join("bin")).unwrap().count(), 1);
}

#[test]
fn release_checksum_path_and_upgrade_replace_the_receipt_and_binary() {
    let old = bytes("1.2.3");
    let old_checksums = format!("{}  {ASSET}\n", digest(&old)).into_bytes();
    let old_server = fixture(
        TAG,
        &[(ASSET, old.clone()), ("checksums.txt", old_checksums)],
    );
    let root = tempfile::tempdir().expect("managed root");
    let old_source = source(&old_server);
    let old_tool = description(
        Some(TAG),
        None,
        DigestDeclaration::ReleaseChecksums {
            asset: "checksums.txt",
        },
    );
    install_tool(
        &old_tool,
        root.path(),
        &old_source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect("first install succeeds");

    let new = bytes("2.0.0");
    let new_tag = "v2.0.0";
    let new_checksums = format!("{}  {ASSET}\n", digest(&new)).into_bytes();
    let new_server = fixture(
        new_tag,
        &[(ASSET, new.clone()), ("checksums.txt", new_checksums)],
    );
    let new_source = source(&new_server);
    let new_tool = description(
        Some(new_tag),
        None,
        DigestDeclaration::ReleaseChecksums {
            asset: "checksums.txt",
        },
    );
    let receipt = install_tool(
        &new_tool,
        root.path(),
        &new_source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect("upgrade succeeds");

    assert_eq!(receipt.version(), "2.0.0");
    assert_eq!(receipt.replaced_version(), Some("1.2.3"));
    assert_eq!(fs::read(root.path().join("bin").join(BINARY)).unwrap(), new);
    let stored: serde_json::Value = serde_json::from_slice(
        &fs::read(root.path().join("state/tool-provision/example-tool.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["version"], "2.0.0");
    assert_eq!(stored["release_tag"], new_tag);
    assert!(
        !fs::read_dir(root.path().join("bin"))
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("previous"))
    );
}

#[test]
fn release_floor_refuses_before_any_download() {
    let binary = bytes("0.9.0");
    let server = fixture("v0.9.0", &[(ASSET, binary.clone())]);
    let source = source(&server);
    let floor = Version::parse("1.0.0").unwrap();
    let pinned_digest = digest(&binary);
    let tool = description(
        None,
        Some(&floor),
        DigestDeclaration::Pinned(&pinned_digest),
    );
    let root = tempfile::tempdir().expect("managed root");

    let error = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect_err("release is below the floor");
    assert!(matches!(
        error,
        InstallError::Release(ReleaseError::Refused(
            longhorn_tool_provision::ReleaseRefusal::NoCompatibleRelease { .. }
        ))
    ));
    assert_eq!(server.request_paths().len(), 1);
    assert!(!root.path().join("bin").join(BINARY).exists());
}

#[test]
fn exact_pin_keeps_a_newer_installed_version_without_network_access() {
    let installed = bytes("2.0.0");
    let server = FixtureServer::spawn(Vec::new()).expect("empty fixture server starts");
    let source = source(&server);
    let root = tempfile::tempdir().expect("managed root");
    let binary_path = root.path().join("bin").join(BINARY);
    fs::create_dir_all(binary_path.parent().unwrap()).unwrap();
    fs::write(&binary_path, &installed).unwrap();
    let digest_pin = digest(&bytes("1.2.3"));
    let tool = description(Some(TAG), None, DigestDeclaration::Pinned(&digest_pin));

    let receipt = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect("the newer managed tool is retained");

    assert!(receipt.already_current());
    assert_eq!(receipt.version(), "2.0.0");
    assert_eq!(receipt.release_tag(), None);
    assert!(server.request_paths().is_empty());
    assert_eq!(fs::read(binary_path).unwrap(), installed);
}

#[test]
fn staged_version_mismatch_is_refused_before_replacement() {
    let binary = bytes("0.9.0");
    let server = fixture(TAG, &[(ASSET, binary.clone())]);
    let source = source(&server);
    let root = tempfile::tempdir().expect("managed root");
    let digest_pin = digest(&binary);
    let tool = description(Some(TAG), None, DigestDeclaration::Pinned(&digest_pin));

    let error = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect_err("the staged version does not match the release");
    assert!(matches!(
        error,
        InstallError::Refused(InstallRefusal::VersionMismatch { .. })
    ));
    assert!(!root.path().join("bin").join(BINARY).exists());
}

#[test]
fn failed_staged_probe_keeps_the_old_binary_untouched() {
    let old = bytes("1.0.0");
    let broken = b"probe-fail\n".to_vec();
    let server = fixture("v2.0.0", &[(ASSET, broken)]);
    let source = source(&server);
    let root = tempfile::tempdir().expect("managed root");
    let binary_path = root.path().join("bin").join(BINARY);
    fs::create_dir_all(binary_path.parent().unwrap()).unwrap();
    fs::write(&binary_path, &old).unwrap();
    let broken_digest = digest(b"probe-fail\n");
    let tool = description(
        Some("v2.0.0"),
        None,
        DigestDeclaration::Pinned(&broken_digest),
    );

    let error = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect_err("staged version probe fails");
    assert!(matches!(
        error,
        InstallError::Fault(InstallFault::Probe {
            phase: ProbePhase::Staged,
            ..
        })
    ));
    assert_eq!(fs::read(binary_path).unwrap(), old);
}

#[test]
fn failed_post_swap_probe_rolls_back_the_previous_binary() {
    let old = bytes("1.0.0");
    let new = bytes("2.0.0");
    let server = fixture("v2.0.0", &[(ASSET, new.clone())]);
    let source = source(&server);
    let root = tempfile::tempdir().expect("managed root");
    let binary_path = root.path().join("bin").join(BINARY);
    fs::create_dir_all(binary_path.parent().unwrap()).unwrap();
    fs::write(&binary_path, &old).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_runner = Arc::clone(&calls);
    let final_path = fs::canonicalize(&binary_path).unwrap();
    let new_for_runner = new.clone();
    let new_digest = digest(&new);
    let runner = TestRunner(move |program: &Path, arguments: &[OsString]| {
        let contents = fs::read(program).unwrap_or_default();
        if program == final_path && contents == new_for_runner {
            calls_for_runner.fetch_add(1, Ordering::SeqCst);
            return Ok(ProcessOutput::new(
                b"new executable failed".to_vec(),
                Vec::new(),
                false,
                Some(7),
            ));
        }
        version_runner().run(program, arguments)
    });
    let tool = description(Some("v2.0.0"), None, DigestDeclaration::Pinned(&new_digest));

    let error = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &runner,
        &redactor(),
        &mut |_| {},
    )
    .expect_err("post-swap probe fails");
    assert!(matches!(
        error,
        InstallError::Fault(InstallFault::Probe {
            phase: ProbePhase::Installed,
            ..
        })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(fs::read(binary_path).unwrap(), old);
    assert_eq!(fs::read_dir(root.path().join("bin")).unwrap().count(), 1);
}

#[test]
fn failed_atomic_receipt_commit_rolls_back_the_binary() {
    let old = bytes("1.0.0");
    let new = bytes("2.0.0");
    let server = fixture("v2.0.0", &[(ASSET, new.clone())]);
    let source = source(&server);
    let root = tempfile::tempdir().expect("managed root");
    let binary_path = root.path().join("bin").join(BINARY);
    fs::create_dir_all(binary_path.parent().unwrap()).unwrap();
    fs::write(&binary_path, &old).unwrap();
    let receipt_path = root.path().join("state/tool-provision/example-tool.json");
    fs::create_dir_all(&receipt_path).unwrap();
    let new_digest = digest(&new);
    let tool = description(Some("v2.0.0"), None, DigestDeclaration::Pinned(&new_digest));

    let error = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect_err("receipt path obstructs atomic commit");
    assert!(matches!(
        error,
        InstallError::Fault(InstallFault::Io { .. })
    ));
    assert_eq!(fs::read(binary_path).unwrap(), old);
    assert!(receipt_path.is_dir());
    assert_eq!(fs::read_dir(root.path().join("bin")).unwrap().count(), 1);
}

#[test]
fn probe_output_is_redacted_and_bounded() {
    let secret = "example-sensitive-token";
    let binary = b"probe-fail\n".to_vec();
    let server = fixture(TAG, &[(ASSET, binary.clone())]);
    let source = source(&server);
    let root = tempfile::tempdir().expect("managed root");
    let pinned_digest = digest(&binary);
    let tool = description(Some(TAG), None, DigestDeclaration::Pinned(&pinned_digest));
    let runner = TestRunner(move |_program: &Path, _arguments: &[OsString]| {
        Ok(ProcessOutput::new(
            format!("{secret}{}", "x".repeat(100_000)).into_bytes(),
            format!("{secret}{}", "y".repeat(100_000)).into_bytes(),
            false,
            Some(1),
        ))
    });
    let redactor = Redactor::new(vec![secret.to_owned()], RedactionRules::exact_only());

    let error = install_tool(
        &tool,
        root.path(),
        &source,
        &TeamVerifier,
        &runner,
        &redactor,
        &mut |_| {},
    )
    .expect_err("probe output is a failure");
    let InstallError::Fault(InstallFault::Probe {
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        ..
    }) = error
    else {
        panic!("expected bounded probe output, got {error:?}");
    };
    assert!(!stdout.contains(secret));
    assert!(!stderr.contains(secret));
    assert!(stdout_truncated);
    assert!(stderr_truncated);
    assert!(stdout.len() <= longhorn_tool_provision::MAX_PROBE_FIELD_BYTES);
    assert!(stderr.len() <= longhorn_tool_provision::MAX_PROBE_FIELD_BYTES);
}

#[test]
fn health_reports_not_installed_installed_and_broken() {
    let root = tempfile::tempdir().expect("managed root");
    let digest_pin = "0".repeat(64);
    let tool = description(None, None, DigestDeclaration::Pinned(&digest_pin));
    let redactor = redactor();
    assert_eq!(
        tool_health(&tool, root.path(), &version_runner(), &redactor),
        ToolHealth::NotInstalled
    );

    let binary = root.path().join("bin").join(BINARY);
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, bytes("3.1.4")).unwrap();
    assert!(matches!(
        tool_health(&tool, root.path(), &version_runner(), &redactor),
        ToolHealth::Installed { version, .. } if version == "3.1.4"
    ));

    fs::write(&binary, b"probe-fail\n").unwrap();
    assert!(matches!(
        tool_health(&tool, root.path(), &version_runner(), &redactor),
        ToolHealth::Broken { .. }
    ));
}

#[test]
fn login_path_parser_ignores_noisy_shell_output_and_cache_refreshes() {
    let parsed = parse_login_path_output(&format!(
        "login banner\n{}\n/first/bin:/second bin\n{}\nmore output\n",
        longhorn_tool_provision::LOGIN_PATH_BEGIN,
        longhorn_tool_provision::LOGIN_PATH_END,
    ))
    .expect("markers contain a PATH");
    assert_eq!(parsed, OsString::from("/first/bin:/second bin"));

    let calls = Arc::new(AtomicUsize::new(0));
    let runner = TestRunner(move |program: &Path, arguments: &[OsString]| {
        assert!(program == Path::new("/bin/bash") || program == Path::new("/bin/zsh"));
        assert_eq!(arguments.first(), Some(&OsString::from("-lc")));
        let index = calls.fetch_add(1, Ordering::SeqCst);
        let path = if index == 0 { "/first" } else { "/second" };
        Ok(ProcessOutput::new(
            format!(
                "startup noise\n{}\n{path}\n{}\n",
                longhorn_tool_provision::LOGIN_PATH_BEGIN,
                longhorn_tool_provision::LOGIN_PATH_END
            )
            .into_bytes(),
            Vec::new(),
            true,
            Some(0),
        ))
    });
    let cache = LoginPathCache::default();
    assert_eq!(cache.get(&runner).value(), OsStr::new("/first"));
    assert_eq!(cache.get(&runner).value(), OsStr::new("/first"));
    assert_eq!(cache.refresh(&runner).value(), OsStr::new("/second"));
}

#[test]
fn login_shell_failure_falls_back_to_process_path_with_detail() {
    let runner = TestRunner(|_program: &Path, _arguments: &[OsString]| {
        Err(longhorn_tool_provision::RunnerError::new(
            "shell unavailable",
        ))
    });
    let found = LoginPathCache::default().get(&runner);
    assert_eq!(
        found.value(),
        std::env::var_os("PATH")
            .as_deref()
            .unwrap_or(OsStr::new("/usr/bin:/bin"))
    );
    assert!(found.detail().is_some());
}

#[cfg(unix)]
#[test]
fn executable_lookup_follows_symlinks_and_keeps_path_order() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};

    let root = tempfile::tempdir().expect("lookup root");
    let first = root.path().join("first");
    let second = root.path().join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    let actual = root.path().join("real-tool");
    fs::write(&actual, bytes("4.0.0")).unwrap();
    let mut permissions = fs::metadata(&actual).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&actual, permissions).unwrap();
    let link = second.join(BINARY);
    symlink(&actual, &link).unwrap();
    let path = std::env::join_paths([first, second.clone()]).unwrap();

    assert_eq!(find_executable_on_path(BINARY, &path), Some(link.clone()));
    assert!(is_executable(&link));

    let managed_root = root.path().join("managed");
    let managed_bin = managed_root.join("bin");
    fs::create_dir_all(&managed_bin).unwrap();
    symlink(&actual, managed_bin.join(BINARY)).unwrap();
    let digest_pin = "0".repeat(64);
    let tool = description(None, None, DigestDeclaration::Pinned(&digest_pin));
    assert!(matches!(
        tool_health(&tool, &managed_root, &version_runner(), &redactor()),
        ToolHealth::Installed { version, .. } if version == "4.0.0"
    ));
}

struct GatedSource {
    inner: HttpReleases,
    started: mpsc::Sender<()>,
    open: Mutex<bool>,
    changed: Condvar,
}

impl ReleaseSource for GatedSource {
    fn resolve_release(&self, query: &ReleaseQuery<'_>) -> Result<Release, ReleaseError> {
        let _ = self.started.send(());
        let mut open = self.open.lock().unwrap();
        while !*open {
            open = self.changed.wait(open).unwrap();
        }
        self.inner.resolve_release(query)
    }

    fn download_asset(
        &self,
        release: &Release,
        asset: &str,
        destination_directory: &Path,
    ) -> Result<PathBuf, ReleaseError> {
        self.inner
            .download_asset(release, asset, destination_directory)
    }
}

#[test]
fn concurrent_installs_of_the_same_tool_are_refused() {
    let binary = bytes("1.2.3");
    let sidecar = format!("{}  {ASSET}\n", digest(&binary)).into_bytes();
    let server = fixture(TAG, &[(ASSET, binary), ("checksums.sha256", sidecar)]);
    let inner = source(&server);
    let (started_tx, started_rx) = mpsc::channel();
    let gated = Arc::new(GatedSource {
        inner,
        started: started_tx,
        open: Mutex::new(false),
        changed: Condvar::new(),
    });
    let root = tempfile::tempdir().expect("managed root");
    let tool = description(
        Some(TAG),
        None,
        DigestDeclaration::Sidecar {
            asset: "checksums.sha256",
        },
    );
    let thread_root = root.path().to_path_buf();
    let thread_source = Arc::clone(&gated);
    let thread_tool = tool;
    let first = thread::spawn(move || {
        install_tool(
            &thread_tool,
            &thread_root,
            thread_source.as_ref(),
            &TeamVerifier,
            &version_runner(),
            &redactor(),
            &mut |_| {},
        )
    });
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("first install reached release resolution");

    let second = install_tool(
        &tool,
        root.path(),
        gated.as_ref(),
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect_err("second install is refused while the first owns the lock");
    assert!(matches!(
        second,
        InstallError::Fault(InstallFault::ConcurrentInstall { .. })
    ));
    *gated.open.lock().unwrap() = true;
    gated.changed.notify_all();
    assert!(first.join().unwrap().is_ok());

    let lock_path = root
        .path()
        .join("state/tool-provision/locks/example-tool.lock");
    let held_lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock_path)
        .expect("persistent install lock exists");
    FileExt::try_lock(&held_lock).expect("test takes the cross-process lock");
    let third = install_tool(
        &tool,
        root.path(),
        gated.as_ref(),
        &TeamVerifier,
        &version_runner(),
        &redactor(),
        &mut |_| {},
    )
    .expect_err("an OS-level lock owner refuses another install");
    assert!(matches!(
        third,
        InstallError::Fault(InstallFault::ConcurrentInstall { .. })
    ));
}
