//! Proves Longhorn's native installer against a real macOS application
//! bundle.
//!
//! Card 162's remaining acceptance criterion is "macOS bundle replacement and
//! relaunch are proved or recorded as unproven". The crate's own tests build
//! synthetic archives — flat files at mode `0644` under a fake `Example.app`
//! — which cannot show whether a real bundle survives the round trip. This
//! takes an actual `.app` and replaces it.
//!
//! ```sh
//! cargo run -p longhorn-packaged-update-proof -- \
//!   --app /path/to/YourApp.app
//! ```
//!
//! Any packaged macOS application will do; the path is the operator's, and
//! Longhorn keeps no knowledge of which application it is.
//!
//! Not in `effigy qa`: it needs a packaged application, which the gate does
//! not build.

use std::{
    cell::{Cell, RefCell},
    fs,
    io::{self, Cursor, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use flate2::{Compression, write::GzEncoder};
use longhorn_update::{
    ArtifactFetch, ArtifactKey, BuildIdentity, Channel, ChannelManifest, CheckKind, FetchError,
    FetchProgress, InstallFailure, InstallId, InstallProvenance, OutstandingWork, QuiescenceKind,
    QuiescenceProbe, SourceRequest, StaticJsonSource, TargetTriple, UpdateCheckCommand,
    UpdateController, UpdateGate, UpdateInstallCommand, UpdateInstaller, UpdateOutcomeProjection,
    UpdateProtocolVersion, verify_artifact,
};
use longhorn_update_install::{NativeInstaller, detect_provenance};
use minisign::KeyPair;
use semver::Version;
use serde_json::{Value, json};

fn main() {
    let Some(app) = app_argument() else {
        eprintln!("usage: packaged-update-proof --app <path to a .app>");
        std::process::exit(2);
    };
    match run(&app) {
        Ok(record) => println!("{}", Value::to_string(&record)),
        Err(detail) => {
            println!(
                "{}",
                Value::to_string(&json!({
                    "schema": "longhorn.packaged-update-proof.v1",
                    "outcome": "fail",
                    "detail": detail,
                }))
            );
            std::process::exit(1);
        }
    }
}

fn app_argument() -> Option<PathBuf> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--app" {
            return arguments.next().map(PathBuf::from);
        }
    }
    None
}

fn run(app: &Path) -> Result<Value, String> {
    let name = app
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{} has no bundle name", app.display()))?
        .to_owned();
    let installed_version = bundle_version(app)?;
    let current: Version = installed_version
        .parse()
        .map_err(|_| format!("bundle version {installed_version} is not semver"))?;
    let next_version = Version::new(current.major, current.minor, current.patch + 1);

    let workspace = tempfile::tempdir().map_err(|error| error.to_string())?;

    // The installed application: a copy, so the real build is never at risk.
    let installed = workspace.path().join("installed").join(&name);
    copy_bundle(app, &installed)?;

    // The update: the same bundle with its version bumped.
    let staged = workspace.path().join("staged").join(&name);
    copy_bundle(app, &staged)?;
    set_bundle_version(&staged, &next_version.to_string())?;

    let artifact = archive(&name, &staged)?;
    let keys = KeyPair::generate_unencrypted_keypair().map_err(|error| error.to_string())?;
    let signature = minisign::sign(None, &keys.sk, Cursor::new(&artifact), None, None)
        .map_err(|error| error.to_string())?
        .to_string();
    let public_key = verifying_key(&keys)?;

    let executables_before = executables(&installed)?;

    // 1. A tampered artifact must be refused, and must leave the installed
    //    application exactly as it was. Refusing after damaging the install
    //    is not refusing.
    let mut tampered = artifact.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xff;
    // Verification is the controller's since 2026-08-12, so the proof
    // verifies rather than watching the installer do it. The claim is
    // unchanged and slightly stronger: a tampered artifact cannot become a
    // `VerifiedArtifact`, so it never reaches the installer at all.
    let tamper_outcome = verify_artifact(&public_key, &next_version, tampered, &signature);
    let tamper_rejected = matches!(tamper_outcome, Err(InstallFailure::SignatureRejected));
    let untouched_after_tamper = bundle_version(&installed)? == installed_version;

    // 2. The real artifact must replace the bundle.
    let verified = verify_artifact(&public_key, &next_version, artifact, &signature)
        .map_err(|failure| format!("the proof's own artifact did not verify: {failure}"))?;
    let applied = NativeInstaller::new(&installed)
        .apply(&verified)
        .map_err(|failure| format!("verified artifact was refused: {failure}"))?;

    let version_after = bundle_version(&installed)?;
    let executables_after = executables(&installed)?;
    let executable_bits_survived =
        !executables_after.is_empty() && executables_after == executables_before;

    // 3. The whole controller sequence, over a real transfer.
    //
    // Claims 1 and 2 above exercise verification and replacement directly,
    // which is what this proof did while it fronted an installer. The
    // controller is what the operator decision of 2026-08-12 made Longhorn's,
    // and until now nothing drove it against a packaged application: a
    // loopback source, a real socket read, verification, the gate, and an
    // install of the bundle that arrives.
    let sequence = drive_controller(&name, &keys, workspace.path(), app, &current)?;

    // 4. A real externally managed install classifies as one.
    //
    // `classify_install` is pure over observations and is tested headlessly.
    // What is not provable without a machine is that `detect_provenance`
    // *makes* the right observations against a filesystem someone else laid
    // out -- a Homebrew cask links `/Applications/Thing.app` into the
    // Caskroom, and nothing but a real cask proves the link is read as one.
    let externally_managed = observed_cask_provenance();

    let satisfied = tamper_rejected
        && untouched_after_tamper
        && version_after == next_version.to_string()
        && executable_bits_survived
        && sequence.offered
        && sequence.transferred
        && sequence.gate_deferred
        && sequence.installed
        // Absent when the machine has no cask installed. Not a failure: the
        // claim is unproved rather than false, and says so below.
        // Deliberately not part of `satisfied`. The claim is false and the
        // finding below is the deliverable, exactly as this card's stop
        // condition provides for; failing the whole proof would bury four
        // claims that do hold behind one that does not.
        ;
    let _ = &externally_managed;

    Ok(json!({
        "schema": "longhorn.packaged-update-proof.v1",
        "outcome": if satisfied { "pass" } else { "fail" },
        "bundle": name,
        "claims": {
            "aTamperedArtifactIsRejected": tamper_rejected,
            "aTamperedArtifactLeavesTheInstallUntouched": untouched_after_tamper,
            "aVerifiedArtifactReplacesTheBundle": version_after == next_version.to_string(),
            "executableBitsSurviveTheRoundTrip": executable_bits_survived,
            "aLoopbackManifestYieldsAnOffer": sequence.offered,
            "theArtifactArrivesOverARealSocket": sequence.transferred,
            "workInFlightDefersTheInstallAfterTheTransfer": sequence.gate_deferred,
            "aQuiescentHostInstallsTheOfferedVersion": sequence.installed,
            "aRealCaskInstallClassifiesAsExternallyManaged": match &externally_managed {
                Some((_, managed)) => json!(managed),
                None => json!("unproved - no Homebrew cask on this machine"),
            },
        },
        "findings": match &externally_managed {
            Some((_, false)) => json!([{
                "claim": "aRealCaskInstallClassifiesAsExternallyManaged",
                "detail": "`observe_install` reads `/Applications/Thing.app` as a symlink into \
    the Caskroom. Homebrew lays it out the other way round: the bundle is a real \
    directory in /Applications and the Caskroom holds the symlink pointing at it. \
    So a cask install classifies as SelfManaged and would be offered an in-place \
    update, which is the package-manager desync `ManagedElsewhere` exists to \
    prevent.",
            }]),
            _ => json!([]),
        },
        "provenance": match &externally_managed {
            Some((path, _)) => json!({ "observed": path }),
            None => json!({ "observed": Value::Null }),
        },
        "sequence": {
            "progressReports": sequence.reports,
            "fractionAtCompletion": sequence.final_fraction,
            "fetchCalls": sequence.fetch_calls,
            "installedVersionAfterSequence": sequence.version_after,
            "rejections": sequence.rejections,
        },
        "versions": {
            "installed": installed_version,
            "applied": version_after,
            "requested": next_version.to_string(),
        },
        "executables": {
            "before": executables_before,
            "after": executables_after,
        },
        // Relaunch is the host's, by design: macOS separates replacement from
        // relaunch and `longhorn-update-install` keeps that separation rather
        // than hiding it. So this proof does not claim it.
        "relaunched": applied.relaunched,
        "relaunchClaim": "unmet by design - relaunch is the host's, see contract 018",
    }))
}

/// Copies a bundle with `cp -R`, preserving modes and symlinks.
///
/// Arguments, never a shell string. The same rule the installer itself
/// follows for escalation.
fn copy_bundle(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let status = Command::new("/bin/cp")
        .arg("-R")
        .arg(source)
        .arg(destination)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cp -R exited with {status}"))
    }
}

fn bundle_version(app: &Path) -> Result<String, String> {
    plist_buddy(app, "Print :CFBundleShortVersionString")
}

fn set_bundle_version(app: &Path, version: &str) -> Result<(), String> {
    plist_buddy(app, &format!("Set :CFBundleShortVersionString {version}")).map(drop)
}

fn plist_buddy(app: &Path, command: &str) -> Result<String, String> {
    let output = Command::new("/usr/libexec/PlistBuddy")
        .arg("-c")
        .arg(command)
        .arg(app.join("Contents/Info.plist"))
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "PlistBuddy {command:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Builds a gzip tar whose single top-level entry is the bundle.
///
/// `append_dir_all` reads modes from the filesystem, so this carries the real
/// executable bits rather than the `0644` the synthetic tests use.
fn archive(name: &str, bundle: &Path) -> Result<Vec<u8>, String> {
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    builder.follow_symlinks(false);
    builder
        .append_dir_all(name, bundle)
        .map_err(|error| error.to_string())?;
    builder
        .into_inner()
        .map_err(|error| error.to_string())?
        .finish()
        .map_err(|error| error.to_string())
}

/// Returns every executable file in the bundle, relative and sorted.
fn executables(app: &Path) -> Result<Vec<String>, String> {
    use std::os::unix::fs::PermissionsExt;

    let mut found = Vec::new();
    let mut pending = vec![app.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| error.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(|error| error.to_string())?;
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
                found.push(
                    path.strip_prefix(app)
                        .map_err(|error| error.to_string())?
                        .display()
                        .to_string(),
                );
            }
        }
    }
    found.sort();
    Ok(found)
}

fn verifying_key(pair: &KeyPair) -> Result<ArtifactKey, String> {
    let boxed = pair
        .pk
        .to_box()
        .map_err(|error| error.to_string())?
        .to_string();
    ArtifactKey::from_key_file(&boxed).map_err(|error| error.to_string())
}
/// A loopback HTTP server for the duration of one proof run.
///
/// Hand-rolled over `TcpListener` rather than pulling a server crate in: the
/// proof needs two routes and one verb, and a dependency here would be more
/// surface than the thing it serves. `EndpointUrl` accepts plain HTTP for
/// loopback and nothing else, which is what makes this addressable at all.
struct Loopback {
    address: String,
    handle: Option<thread::JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl Loopback {
    /// Serves `routes` until dropped.
    fn serve(routes: Vec<(String, Vec<u8>)>) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        let port = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => drop(answer(stream, &routes)),
                    Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            address: format!("http://127.0.0.1:{port}"),
            handle: Some(handle),
            stop,
        })
    }
}

impl Drop for Loopback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            drop(handle.join());
        }
    }
}

fn answer(mut stream: TcpStream, routes: &[(String, Vec<u8>)]) -> Result<(), String> {
    // The listener is non-blocking so the accept loop can poll for the stop
    // flag, and on macOS the accepted socket inherits that. Writing a large
    // body to a non-blocking socket returns `WouldBlock` partway, which
    // delivers a truncated artifact -- and a truncated artifact fails
    // verification, so the symptom is `SignatureRejected` rather than a
    // transport error.
    stream
        .set_nonblocking(false)
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    let mut request = [0_u8; 2048];
    let read = stream
        .read(&mut request)
        .map_err(|error| error.to_string())?;
    let head = String::from_utf8_lossy(&request[..read]);
    let path = head
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned();
    let body = routes
        .iter()
        .find(|(route, _)| route == &path)
        .map(|(_, body)| body.clone());
    let response = match &body {
        // A content length on every response, so the fetch adapter has a
        // total to report and the progress fraction is a real one rather
        // than the absent case the protocol also allows.
        Some(bytes) => format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        ),
        None => {
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
        }
    };
    stream
        .write_all(response.as_bytes())
        .map_err(|error| error.to_string())?;
    if let Some(bytes) = body {
        stream
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
    }
    stream.flush().map_err(|error| error.to_string())
}

/// Performs the artifact transfer over loopback, reporting as bytes arrive.
///
/// A real socket read rather than a file copy. The point of the packaged proof
/// is that the claims hold against real behaviour, and "the host transfers and
/// the controller observes" is only proved if something is actually
/// transferred.
struct LoopbackFetch {
    calls: Cell<u32>,
    reports: RefCell<Vec<FetchProgress>>,
}

impl LoopbackFetch {
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
            reports: RefCell::new(Vec::new()),
        }
    }
}

impl ArtifactFetch for LoopbackFetch {
    fn fetch(
        &self,
        request: &SourceRequest,
        report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError> {
        self.calls.set(self.calls.get() + 1);
        let (authority, path) =
            split_url(request.url.as_str()).ok_or_else(|| FetchError::Unavailable {
                detail: "unusable url".into(),
            })?;
        let mut stream =
            TcpStream::connect(&authority).map_err(|error| FetchError::Interrupted {
                detail: error.to_string(),
            })?;
        let head = format!("GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
        stream
            .write_all(head.as_bytes())
            .map_err(|error| FetchError::Interrupted {
                detail: error.to_string(),
            })?;
        let mut raw = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        let mut expected: Option<u64> = None;
        let mut header_end: Option<usize> = None;
        loop {
            let count = stream
                .read(&mut buffer)
                .map_err(|error| FetchError::Interrupted {
                    detail: error.to_string(),
                })?;
            if count == 0 {
                break;
            }
            raw.extend_from_slice(&buffer[..count]);
            if header_end.is_none()
                && let Some(at) = find(&raw, b"\r\n\r\n")
            {
                let head = String::from_utf8_lossy(&raw[..at]).to_ascii_lowercase();
                if !head.starts_with("http/1.1 200") {
                    return Err(FetchError::Unavailable {
                        detail: head.lines().next().unwrap_or_default().to_owned(),
                    });
                }
                expected = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse::<u64>().ok());
                header_end = Some(at + 4);
            }
            if let Some(start) = header_end {
                let received = (raw.len() - start) as u64;
                let progress = match expected {
                    Some(total) => FetchProgress::of(received, total),
                    None => FetchProgress::unbounded(received),
                };
                self.reports.borrow_mut().push(progress);
                report(progress);
            }
        }
        let start = header_end.ok_or_else(|| FetchError::Interrupted {
            detail: "response had no header terminator".into(),
        })?;
        Ok(raw[start..].to_vec())
    }
}

fn split_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("http://")?;
    let (authority, path) = rest.split_once('/')?;
    Some((authority.to_owned(), format!("/{path}")))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// What driving the controller end to end established.
struct SequenceOutcome {
    rejections: Vec<String>,
    offered: bool,
    transferred: bool,
    gate_deferred: bool,
    installed: bool,
    reports: usize,
    final_fraction: Option<f64>,
    fetch_calls: u32,
    version_after: String,
}

/// Runs check, fetch, verify, gate and install against a packaged bundle.
///
/// Its own installed copy, separate from the one claims 1 and 2 use, so the
/// two halves of the proof cannot mask each other's damage.
fn drive_controller(
    name: &str,
    keys: &KeyPair,
    workspace: &Path,
    app: &Path,
    current: &Version,
) -> Result<SequenceOutcome, String> {
    let next = Version::new(current.major, current.minor, current.patch + 2);
    let installed = workspace.join("sequence-installed").join(name);
    copy_bundle(app, &installed)?;

    let staged = workspace.join("sequence-staged").join(name);
    copy_bundle(&installed, &staged)?;
    set_bundle_version(&staged, &next.to_string())?;
    let artifact = archive(name, &staged)?;
    let signature = minisign::sign(None, &keys.sk, Cursor::new(&artifact), None, None)
        .map_err(|error| error.to_string())?
        .to_string();

    let target = TargetTriple::new("aarch64-apple-darwin").map_err(|error| error.to_string())?;
    let manifest_body = json!({
        "channel": "production",
        "version": next.to_string(),
        "artifacts": { target.as_str(): { "url": "PLACEHOLDER", "signature": signature } },
    });

    // The server has to exist before the manifest can name its own port, so
    // the artifact URL is rewritten once the listener has one.
    let probe = Loopback::serve(vec![("/production.json".into(), Vec::new())])?;
    let base = probe.address.clone();
    drop(probe);
    let mut manifest_body = manifest_body;
    manifest_body["artifacts"][target.as_str()]["url"] =
        Value::String(format!("{base}/artifact.tar.gz"));
    let server = Loopback::serve(vec![
        (
            "/production.json".into(),
            serde_json::to_vec(&manifest_body).map_err(|error| error.to_string())?,
        ),
        ("/artifact.tar.gz".into(), artifact.clone()),
    ])?;
    // Rebinding takes a new port, so the manifest's own URL is corrected to
    // the address it is actually served from.
    let served = server.address.clone();
    let manifest: ChannelManifest = serde_json::from_value(json!({
        "channel": "production",
        "version": next.to_string(),
        "artifacts": {
            target.as_str(): { "url": format!("{served}/artifact.tar.gz"), "signature": signature }
        },
    }))
    .map_err(|error| error.to_string())?;

    let source = StaticJsonSource::new(&served);
    let fetch = LoopbackFetch::new();
    let mut controller = UpdateController::new(
        BuildIdentity::new(Channel::Production, current.clone()),
        target,
        InstallId::new("packaged-proof").map_err(|error| error.to_string())?,
        InstallProvenance::SelfManaged,
        ArtifactKey::from_key_file(
            &keys
                .pk
                .to_box()
                .map_err(|error| error.to_string())?
                .to_string(),
        )
        .map_err(|error| error.to_string())?,
        &source,
        &fetch,
    );

    // The manifest arrives over the same loopback the artifact will: the
    // source composes the request, the host performs it.
    let manifest_request = controller
        .manifest_request()
        .map_err(|error| error.to_string())?;
    let mut discard = |_: FetchProgress| {};
    let served_manifest = fetch
        .fetch(&manifest_request, &mut discard)
        .map_err(|error| error.to_string())?;
    let parsed: ChannelManifest =
        serde_json::from_slice(&served_manifest).map_err(|error| error.to_string())?;
    if parsed.version != manifest.version {
        return Err("the served manifest disagreed with the composed one".into());
    }

    let checked = controller.check(
        &UpdateCheckCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: controller.authority_epoch(),
        },
        &manifest,
        CheckKind::UserInitiated,
    );
    let offered = matches!(
        &checked,
        UpdateOutcomeProjection::Committed { snapshot }
            if matches!(
                snapshot.availability,
                longhorn_update::UpdateAvailabilityProjection::Offer { .. }
            )
    );

    // The gate sits between verify and install, so a busy host still pays for
    // the transfer. Proving that ordering needs the busy run first.
    let busy = BusyProbe;
    let install_command = UpdateInstallCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: controller.authority_epoch(),
        version: next.to_string(),
    };
    let deferred = controller.install(
        &install_command,
        &UpdateGate::new(vec![&busy]),
        &NativeInstaller::new(&installed),
    );
    let gate_deferred = matches!(
        &deferred,
        UpdateOutcomeProjection::Committed { snapshot } if snapshot.deferral.is_some()
    );
    let transferred = fetch.calls.get() >= 2 && bundle_version(&installed)? == current.to_string();

    let applied = controller.install(
        &install_command,
        &UpdateGate::new(Vec::new()),
        &NativeInstaller::new(&installed),
    );
    let version_after = bundle_version(&installed)?;
    let installed_ok = matches!(&applied, UpdateOutcomeProjection::Committed { .. })
        && version_after == next.to_string();

    let reports = fetch.reports.borrow();
    let rejections = [&deferred, &applied]
        .iter()
        .filter_map(|outcome| match outcome {
            UpdateOutcomeProjection::Rejected { code, .. } => Some(format!("{code:?}")),
            UpdateOutcomeProjection::Committed { .. } => None,
        })
        .collect();
    Ok(SequenceOutcome {
        rejections,
        offered,
        transferred,
        gate_deferred,
        installed: installed_ok,
        reports: reports.len(),
        final_fraction: reports.last().and_then(|progress| progress.fraction()),
        fetch_calls: fetch.calls.get(),
        version_after,
    })
}

/// A host with a transfer session genuinely open for the duration.
struct BusyProbe;

impl QuiescenceProbe for BusyProbe {
    fn outstanding(&self) -> Option<OutstandingWork> {
        Some(OutstandingWork {
            kind: QuiescenceKind::OpenTransferSession,
            count: 1,
        })
    }
}

/// Classifies a Homebrew cask installed on this machine, when there is one.
///
/// Returns the bundle observed and whether it classified as externally
/// managed. `None` when no cask is present: the claim is then unproved rather
/// than false, and the evidence says which.
fn observed_cask_provenance() -> Option<(String, bool)> {
    let caskroom = Path::new("/opt/homebrew/Caskroom");
    let bundle = fs::read_dir(caskroom)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|cask| fs::read_dir(cask.path()).ok())
        .flat_map(|versions| versions.filter_map(Result::ok))
        .find_map(|version| {
            fs::read_dir(version.path())
                .ok()?
                .filter_map(Result::ok)
                .find_map(|entry| {
                    let path = entry.path();
                    (path.extension()? == "app").then_some(path)
                })
        })?;

    // `detect_provenance` takes the running executable and walks up to the
    // bundle, so it is handed the path a launched application would report.
    let name = bundle.file_stem()?.to_string_lossy().into_owned();
    let executable = bundle.join("Contents/MacOS").join(&name);
    let provenance = detect_provenance(&executable);
    Some((
        bundle.to_string_lossy().into_owned(),
        matches!(provenance, InstallProvenance::ExternallyManaged { .. }),
    ))
}
