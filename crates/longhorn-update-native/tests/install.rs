//! Native installer evidence, including the shared conformance suite.
//!
//! Keys, archives and signatures are generated in-test rather than committed
//! as fixtures, so nothing here can drift from the signing format it claims
//! to accept.

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use flate2::{Compression, write::GzEncoder};
use longhorn_update::{ConformanceFixtures, InstallFailure, UpdateInstaller, run_conformance};
use longhorn_update_native::{NativeInstaller, NoPrivilegedReplace, PrivilegedReplace};
use minisign::KeyPair;
use minisign_verify::PublicKey;
use semver::Version;
use tempfile::TempDir;

/// One keypair for the whole run, generated without a password.
fn keypair() -> KeyPair {
    KeyPair::generate_unencrypted_keypair().unwrap()
}

fn verifying_key(pair: &KeyPair) -> PublicKey {
    // The public-key box is the `.pub` file: a comment line then the key.
    let boxed = pair.pk.to_box().unwrap().to_string();
    let encoded = boxed.trim().lines().last().unwrap().trim();
    PublicKey::from_base64(encoded).unwrap()
}

fn sign(pair: &KeyPair, data: &[u8]) -> String {
    minisign::sign(None, &pair.sk, Cursor::new(data), None, None)
        .unwrap()
        .to_string()
}

/// Builds a gzip tar whose single top-level entry is `root`.
fn archive(root: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    for (name, contents) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("{root}/{name}"), *contents)
            .unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

/// Builds an archive whose entry escapes the destination.
///
/// The header name is written directly because `tar::Builder` refuses to
/// create such a path -- which is the point: only a hostile producer emits
/// one, so the installer must not trust that its input came from `Builder`.
fn traversing_archive() -> Vec<u8> {
    let contents = b"nope";
    let mut header = tar::Header::new_gnu();
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    let name = b"../escaped";
    header.as_gnu_mut().unwrap().name[..name.len()].copy_from_slice(name);
    header.set_cksum();

    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    builder.append(&header, &contents[..]).unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}

fn bundle() -> Vec<u8> {
    archive("Example.app", &[("Contents/marker", b"new")])
}

struct Fixtures {
    pair: KeyPair,
}

impl ConformanceFixtures for Fixtures {
    fn version(&self) -> Version {
        Version::parse("1.3.0").unwrap()
    }

    fn valid(&self) -> (Vec<u8>, String) {
        let bytes = bundle();
        let signature = sign(&self.pair, &bytes);
        (bytes, signature)
    }

    fn tampered(&self) -> (Vec<u8>, String) {
        let mut bytes = bundle();
        let signature = sign(&self.pair, &bytes);
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        (bytes, signature)
    }

    fn signed_but_unusable(&self) -> Option<(Vec<u8>, String)> {
        let bytes = b"correctly signed, but not a gzip tar".to_vec();
        let signature = sign(&self.pair, &bytes);
        Some((bytes, signature))
    }
}

struct Fixture {
    _temp: TempDir,
    target: PathBuf,
    fixtures: Fixtures,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("Example.app");
        fs::create_dir_all(target.join("Contents")).unwrap();
        fs::write(target.join("Contents/marker"), b"old").unwrap();
        Self {
            target,
            fixtures: Fixtures { pair: keypair() },
            _temp: temp,
        }
    }

    fn installer(&self) -> NativeInstaller<NoPrivilegedReplace> {
        NativeInstaller::new(verifying_key(&self.fixtures.pair), &self.target)
    }

    fn marker(&self) -> String {
        fs::read_to_string(self.target.join("Contents/marker")).unwrap()
    }

    fn parent(&self) -> &Path {
        self.target.parent().unwrap()
    }
}

#[test]
fn the_native_installer_satisfies_the_shared_conformance_suite() {
    let fixture = Fixture::new();

    let outcomes = run_conformance(&fixture.installer(), &fixture.fixtures);

    for outcome in &outcomes {
        assert!(outcome.satisfied, "{}: {:?}", outcome.claim, outcome.detail);
    }
    assert_eq!(outcomes.len(), 4);
}

#[test]
fn a_verified_artifact_replaces_the_installed_application() {
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();

    fixture
        .installer()
        .apply(&fixture.fixtures.version(), &bytes, &signature)
        .expect("a verified artifact installs");

    assert_eq!(fixture.marker(), "new");
}

#[test]
fn a_tampered_artifact_leaves_the_installed_application_untouched() {
    // Refusing is necessary; refusing without having already disturbed the
    // install is what makes it safe.
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.tampered();

    assert_eq!(
        fixture
            .installer()
            .apply(&fixture.fixtures.version(), &bytes, &signature),
        Err(InstallFailure::SignatureRejected)
    );
    assert_eq!(fixture.marker(), "old");
}

#[test]
fn a_signed_archive_escaping_the_destination_is_refused() {
    // A signature proves origin, not good intent. An archive stays untrusted
    // input after it verifies, so traversal is checked on a *signed* archive.
    let fixture = Fixture::new();
    // tar's builder refuses to *write* a traversing path, so the header name
    // is set directly -- which is exactly what a hostile archive would do.
    let escaping = traversing_archive();
    let signature = sign(&fixture.fixtures.pair, &escaping);

    let outcome = fixture
        .installer()
        .apply(&fixture.fixtures.version(), &escaping, &signature);

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a traversal refusal, found {outcome:?}"
    );
    assert!(!fixture.parent().join("escaped").exists());
    assert_eq!(fixture.marker(), "old");
}

#[test]
fn escalation_is_not_attempted_when_the_target_is_writable() {
    // An application that never opted into escalation must not prompt for a
    // password it did not ask to need.
    struct Forbidden;

    impl PrivilegedReplace for Forbidden {
        fn replace(&self, _staged: &Path, _target: &Path) -> Result<(), String> {
            panic!("escalation must not be attempted on a writable target");
        }
    }

    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();

    fixture
        .installer()
        .with_escalation(Forbidden)
        .apply(&fixture.fixtures.version(), &bytes, &signature)
        .expect("a writable target needs no escalation");
}

#[test]
fn the_default_escalation_declines_rather_than_prompting() {
    assert!(
        NoPrivilegedReplace
            .replace(Path::new("/staged"), Path::new("/target"))
            .is_err()
    );
}

#[test]
fn a_failed_install_leaves_no_staging_directory_behind() {
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.signed_but_unusable().unwrap();

    drop(
        fixture
            .installer()
            .apply(&fixture.fixtures.version(), &bytes, &signature),
    );

    let leftovers: Vec<_> = fs::read_dir(fixture.parent())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".longhorn-update")
        })
        .collect();
    assert!(leftovers.is_empty(), "staging directory was left behind");
}

#[test]
fn relaunch_is_left_to_the_host() {
    // macOS separates replacement from relaunch. Longhorn keeps that
    // separation rather than hiding it, so the caller orders teardown.
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();

    let applied = fixture
        .installer()
        .apply(&fixture.fixtures.version(), &bytes, &signature)
        .unwrap();

    assert!(!applied.relaunched);
}
