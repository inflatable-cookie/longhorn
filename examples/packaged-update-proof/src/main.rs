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
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

use flate2::{Compression, write::GzEncoder};
use longhorn_update::{ArtifactKey, InstallFailure, UpdateInstaller, verify_artifact};
use longhorn_update_install::NativeInstaller;
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

    let satisfied = tamper_rejected
        && untouched_after_tamper
        && version_after == next_version.to_string()
        && executable_bits_survived;

    Ok(json!({
        "schema": "longhorn.packaged-update-proof.v1",
        "outcome": if satisfied { "pass" } else { "fail" },
        "bundle": name,
        "claims": {
            "aTamperedArtifactIsRejected": tamper_rejected,
            "aTamperedArtifactLeavesTheInstallUntouched": untouched_after_tamper,
            "aVerifiedArtifactReplacesTheBundle": version_after == next_version.to_string(),
            "executableBitsSurviveTheRoundTrip": executable_bits_survived,
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
