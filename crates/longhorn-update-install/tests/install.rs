//! Native installer evidence, including the shared conformance suite.
//!
//! Keys, archives and signatures are generated in-test rather than committed
//! as fixtures, so nothing here can drift from the signing format it claims
//! to accept.

#![cfg(unix)]

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use flate2::{Compression, write::GzEncoder};
use longhorn_update::{
    ArtifactKey, ConformanceFixtures, InstallFailure, UpdateInstaller, VerifiedArtifact,
    run_conformance, verify_artifact,
};
use longhorn_update_install::{NativeInstaller, NoPrivilegedReplace, PrivilegedReplace};
use minisign::KeyPair;
use semver::Version;
use tempfile::TempDir;

/// One keypair for the whole run, generated without a password.
fn keypair() -> KeyPair {
    KeyPair::generate_unencrypted_keypair().unwrap()
}

fn verifying_key(pair: &KeyPair) -> ArtifactKey {
    // The public-key box is the `.pub` file: a comment line then the key.
    ArtifactKey::from_key_file(&pair.pk.to_box().unwrap().to_string()).unwrap()
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

/// Appends a link entry; the entry type decides hard link or symlink.
///
/// `Builder` does not refuse hostile link *targets* — only paths — so the
/// escaping cases below build exactly what a hostile producer would emit.
fn append_link(
    builder: &mut tar::Builder<GzEncoder<Vec<u8>>>,
    kind: tar::EntryType,
    path: &str,
    target: &str,
) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(kind);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_cksum();
    builder.append_link(&mut header, path, target).unwrap();
}

fn finish(builder: tar::Builder<GzEncoder<Vec<u8>>>) -> Vec<u8> {
    builder.into_inner().unwrap().finish().unwrap()
}

fn gzip_builder() -> tar::Builder<GzEncoder<Vec<u8>>> {
    tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()))
}

/// A symlink pointing out of the tree, followed by a file written through it.
#[cfg(unix)]
fn symlink_escape_archive() -> Vec<u8> {
    let mut builder = gzip_builder();
    append_link(
        &mut builder,
        tar::EntryType::Symlink,
        "Example.app/escape",
        "../../..",
    );
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "Example.app/escape/payload", &b"nope"[..])
        .unwrap();
    finish(builder)
}

/// A symlink whose target is absolute.
#[cfg(unix)]
fn absolute_symlink_archive() -> Vec<u8> {
    let mut builder = gzip_builder();
    append_link(
        &mut builder,
        tar::EntryType::Symlink,
        "Example.app/escape",
        "/etc",
    );
    finish(builder)
}

/// A hard link whose target escapes the tree.
#[cfg(unix)]
fn hardlink_escape_archive() -> Vec<u8> {
    let mut builder = gzip_builder();
    append_link(
        &mut builder,
        tar::EntryType::Link,
        "Example.app/pwned",
        "../../../../etc/passwd",
    );
    finish(builder)
}

/// A hard link to a member the archive never writes.
///
/// Every payload in this stream is complete and every path is bounded: the
/// fault is that the member the link names does not exist, which tar meets
/// only when it creates the link.
#[cfg(unix)]
fn dangling_hardlink_archive() -> Vec<u8> {
    let mut builder = gzip_builder();
    append_link(
        &mut builder,
        tar::EntryType::Link,
        "Example.app/alias",
        "Example.app/Contents/missing",
    );
    let mut header = tar::Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "Example.app/Contents/marker", &b"new"[..])
        .unwrap();
    finish(builder)
}

/// The archive's single top-level entry is itself a symlink: renamed onto
/// the install target, the application would become a pointer.
#[cfg(unix)]
fn symlink_root_archive() -> Vec<u8> {
    let mut builder = gzip_builder();
    append_link(
        &mut builder,
        tar::EntryType::Symlink,
        "Example.app",
        "elsewhere",
    );
    finish(builder)
}

/// A bundle with a legitimate in-tree relative symlink, as real application
/// bundles carry (`Contents/Current -> Versions/A`).
#[cfg(unix)]
fn bundle_with_symlink() -> Vec<u8> {
    let mut builder = gzip_builder();
    append_link(
        &mut builder,
        tar::EntryType::Symlink,
        "Example.app/Contents/Current",
        "Versions/A",
    );
    let mut header = tar::Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "Example.app/Contents/marker", &b"new"[..])
        .unwrap();
    finish(builder)
}

fn bundle() -> Vec<u8> {
    archive("Example.app", &[("Contents/marker", b"new")])
}

/// A signed archive whose stream ends before the payload its header declares.
///
/// The gzip member is intact, so the decoder reports no fault: only the tar
/// structure is short, which is the shape a transfer that stopped early
/// leaves behind.
fn truncated_archive() -> Vec<u8> {
    let payload = vec![b'x'; 4096];
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(payload.len() as u64);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    builder
        .append_data(&mut header, "Example.app/Contents/marker", &payload[..])
        .unwrap();
    let mut plain = builder.into_inner().unwrap();
    plain.truncate(plain.len() - 3000);

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, &plain).unwrap();
    encoder.finish().unwrap()
}

struct Fixtures {
    pair: KeyPair,
}

impl ConformanceFixtures for Fixtures {
    fn version(&self) -> Version {
        Version::parse("1.3.0").unwrap()
    }

    fn key(&self) -> ArtifactKey {
        verifying_key(&self.pair)
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

    #[cfg(unix)]
    fn signed_but_escaping(&self) -> Vec<(Vec<u8>, String)> {
        [
            symlink_escape_archive(),
            absolute_symlink_archive(),
            hardlink_escape_archive(),
            symlink_root_archive(),
        ]
        .into_iter()
        .map(|bytes| {
            let signature = sign(&self.pair, &bytes);
            (bytes, signature)
        })
        .collect()
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
        let target = temp.path().canonicalize().unwrap().join("Example.app");
        fs::create_dir_all(target.join("Contents")).unwrap();
        fs::write(target.join("Contents/marker"), b"old").unwrap();
        Self {
            target,
            fixtures: Fixtures { pair: keypair() },
            _temp: temp,
        }
    }

    fn installer(&self) -> NativeInstaller<NoPrivilegedReplace> {
        NativeInstaller::new(&self.target)
    }

    fn marker(&self) -> String {
        fs::read_to_string(self.target.join("Contents/marker")).unwrap()
    }

    fn parent(&self) -> &Path {
        self.target.parent().unwrap()
    }

    fn workspace(&self) -> PathBuf {
        self.parent().join(".--.longhorn-install-Example.app")
    }
}

#[test]
fn the_native_installer_satisfies_the_shared_conformance_suite() {
    let fixture = Fixture::new();

    let outcomes = run_conformance(&fixture.installer(), &fixture.fixtures);

    for outcome in &outcomes {
        assert!(outcome.satisfied, "{}: {:?}", outcome.claim, outcome.detail);
    }
    // Four base claims plus one per hostile-archive fixture.
    assert_eq!(outcomes.len(), 8);
}

#[test]
fn a_verified_artifact_replaces_the_installed_application() {
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();

    fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .expect("a verified artifact installs");

    assert_eq!(fixture.marker(), "new");
}

#[test]
fn a_tampered_artifact_never_reaches_the_installer() {
    // This used to hand the tampered bytes to `apply` and assert it refused
    // them. It cannot: `apply` takes a `VerifiedArtifact`, and there is no way
    // to make one from bytes that do not verify. The install is untouched
    // because nothing was called, which is a stronger statement than the one
    // this test used to make.
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.tampered();

    assert_eq!(
        verify_artifact(
            &fixture.fixtures.key(),
            &fixture.fixtures.version(),
            bytes,
            &signature,
        ),
        Err(InstallFailure::SignatureRejected)
    );
    assert_eq!(fixture.marker(), "old");
}

#[test]
fn an_extra_top_level_entry_refuses_the_entire_update() {
    let fixture = Fixture::new();
    let bytes = archive(
        ".",
        &[
            ("Example.app/Contents/marker", b"new"),
            ("extra.txt", b"unexpected"),
        ],
    );
    let signature = sign(&fixture.fixtures.pair, &bytes);
    let failure = fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .unwrap_err();
    assert!(matches!(failure, InstallFailure::MalformedArtifact { .. }));
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
        .apply(&verified(&fixture, escaping, &signature));

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a traversal refusal, found {outcome:?}"
    );
    assert!(!fixture.parent().join("escaped").exists());
    assert_eq!(fixture.marker(), "old");
}

#[test]
#[cfg(unix)]
fn a_planted_symlink_writes_nothing_outside_the_install() {
    // The shared suite asserts the refusal class; this asserts the
    // filesystem: nothing landed outside the target, and the install is
    // untouched.
    let fixture = Fixture::new();
    let escaping = symlink_escape_archive();
    let signature = sign(&fixture.fixtures.pair, &escaping);

    let outcome = fixture
        .installer()
        .apply(&verified(&fixture, escaping, &signature));

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a link refusal, found {outcome:?}"
    );
    assert!(!fixture.parent().join("payload").exists());
    assert_eq!(fixture.marker(), "old");
}

#[test]
#[cfg(unix)]
fn an_in_tree_symlink_is_part_of_a_valid_bundle() {
    // Real application bundles carry relative symlinks; bounded extraction
    // rejects escaping links, not links.
    let fixture = Fixture::new();
    let bytes = bundle_with_symlink();
    let signature = sign(&fixture.fixtures.pair, &bytes);

    fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .expect("a bundle with an in-tree symlink installs");

    assert_eq!(fixture.marker(), "new");
    let link = fs::read_link(fixture.target.join("Contents/Current")).unwrap();
    assert_eq!(link, Path::new("Versions/A"));
}

#[test]
fn a_displaced_legacy_bundle_is_left_for_manual_recovery() {
    let fixture = Fixture::new();
    let backup = fixture.target.with_extension("longhorn-previous");
    fs::rename(&fixture.target, &backup).unwrap();
    let (bytes, signature) = fixture.fixtures.valid();

    let error = fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .expect_err("legacy backup ownership must be confirmed manually");

    assert!(error.to_string().contains(&*backup.to_string_lossy()));
    assert_eq!(fs::read(backup.join("Contents/marker")).unwrap(), b"old");
    assert!(!fixture.target.exists());
}

#[test]
fn matching_workspace_names_do_not_authorize_recovery_of_an_unrelated_directory() {
    let fixture = Fixture::new();
    let marker = fixture.workspace().join("previous/marker");
    fs::create_dir_all(marker.parent().unwrap()).unwrap();
    fs::write(&marker, b"keep").unwrap();
    fs::write(
        fixture.workspace().join("longhorn-update-install-v1.lock"),
        b"",
    )
    .unwrap();
    fs::rename(&fixture.target, fixture.parent().join("original")).unwrap();
    let (bytes, signature) = fixture.fixtures.valid();

    let error = fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .expect_err("an existing workspace needs the ownership marker");

    assert!(error.to_string().contains("unrecognized"));
    assert_eq!(fs::read(&marker).unwrap(), b"keep");
    assert!(!fixture.target.exists());
}

#[test]
fn a_signed_truncated_archive_is_malformed_not_retryable() {
    // A signed archive is still untrusted input, and a truncated one is
    // terminal: reporting it as a retryable failure replays the same bad bytes
    // on every attempt and keeps its staged copy forever.
    let fixture = Fixture::new();
    let truncated = truncated_archive();
    let signature = sign(&fixture.fixtures.pair, &truncated);

    let outcome = fixture
        .installer()
        .apply(&verified(&fixture, truncated, &signature));

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a malformed-artifact refusal, found {outcome:?}"
    );
    assert_eq!(fixture.marker(), "old");
}

#[test]
fn a_signed_archive_with_a_truncated_stream_is_malformed_not_retryable() {
    // The stream ends inside an entry's data, after its header has been read:
    // the decoder faults while the entry is being written, which is the shape
    // a transfer that stopped early leaves. The payload is incompressible so
    // the cut lands inside the deflate body rather than after it.
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let filler: Vec<u8> = (0..256 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    let fixture = Fixture::new();
    let mut cut = archive(
        "Example.app",
        &[("Contents/marker", b"new"), ("Contents/filler", &filler)],
    );
    cut.truncate(cut.len() * 6 / 10);
    let signature = sign(&fixture.fixtures.pair, &cut);

    let outcome = fixture
        .installer()
        .apply(&verified(&fixture, cut, &signature));

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a malformed-artifact refusal, found {outcome:?}"
    );
    assert_eq!(fixture.marker(), "old");
}

#[test]
#[cfg(unix)]
fn a_signed_hard_link_to_a_missing_member_is_malformed_not_retryable() {
    // A structurally incomplete archive: the stream is whole, and the link
    // names a member that is not in it. Reading the stream again cannot find
    // that, so it is checked where the link is admitted, and stays terminal.
    let fixture = Fixture::new();
    let bytes = dangling_hardlink_archive();
    let signature = sign(&fixture.fixtures.pair, &bytes);

    let outcome = fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature));

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a malformed-artifact refusal, found {outcome:?}"
    );
    assert_eq!(fixture.marker(), "old");
}

#[test]
#[cfg(unix)]
fn a_signed_hard_link_through_a_file_is_malformed_not_retryable() {
    let mut builder = gzip_builder();
    let mut header = tar::Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "Example.app/Contents/marker", &b"new"[..])
        .unwrap();
    append_link(
        &mut builder,
        tar::EntryType::Link,
        "Example.app/alias",
        "Example.app/Contents/marker/child",
    );
    let bytes = finish(builder);
    let fixture = Fixture::new();
    let signature = sign(&fixture.fixtures.pair, &bytes);

    let outcome = fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature));

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a malformed-artifact refusal, found {outcome:?}"
    );
    assert_eq!(fixture.marker(), "old");
}

#[test]
#[cfg(unix)]
fn a_signed_hard_link_to_a_written_member_installs() {
    // The refusal above must not turn legitimate hard links into malformed
    // archives: a link to a member the archive does contain is part of a
    // bundle.
    let mut builder = gzip_builder();
    let mut header = tar::Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "Example.app/Contents/marker", &b"new"[..])
        .unwrap();
    append_link(
        &mut builder,
        tar::EntryType::Link,
        "Example.app/Contents/alias",
        "Example.app/Contents/marker",
    );
    let bytes = finish(builder);
    let fixture = Fixture::new();
    let signature = sign(&fixture.fixtures.pair, &bytes);

    fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .expect("a hard link to a written member installs");

    assert_eq!(fixture.marker(), "new");
    assert_eq!(
        fs::read(fixture.target.join("Contents/alias")).unwrap(),
        b"new"
    );
}

#[test]
fn an_archive_overstaying_the_extraction_quota_is_refused() {
    // The quota reads declared sizes, so a header claiming five gigabytes for
    // a four-byte payload trips it before a byte is unpacked.
    let fixture = Fixture::new();
    let contents = b"tiny";
    let mut header = tar::Header::new_gnu();
    header.set_size(5 * 1024 * 1024 * 1024 + 1);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    let name = b"Example.app/Contents/big";
    header.as_gnu_mut().unwrap().name[..name.len()].copy_from_slice(name);
    header.set_cksum();
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    builder.append(&header, &contents[..]).unwrap();
    let overstating = builder.into_inner().unwrap().finish().unwrap();
    let signature = sign(&fixture.fixtures.pair, &overstating);

    let outcome = fixture
        .installer()
        .apply(&verified(&fixture, overstating, &signature));

    assert!(
        matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
        "expected a quota refusal, found {outcome:?}"
    );
    assert_eq!(fixture.marker(), "old");
}

#[test]
fn apply_leaves_unowned_siblings_in_place() {
    let fixture = Fixture::new();
    let unrelated = fixture.parent().join(".longhorn-update-9.9.9-other");
    let legacy_backup = fixture.target.with_extension("longhorn-previous");
    for sibling in [&unrelated, &legacy_backup] {
        fs::create_dir(sibling).unwrap();
        fs::write(sibling.join("marker"), b"keep").unwrap();
    }

    let (bytes, signature) = fixture.fixtures.valid();
    fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .expect("a verified artifact installs");

    for sibling in [&unrelated, &legacy_backup] {
        assert_eq!(fs::read(sibling.join("marker")).unwrap(), b"keep");
    }
    assert_eq!(fixture.marker(), "new");
}

#[test]
fn escalation_is_not_attempted_when_the_target_is_writable() {
    // An application that never opted into escalation must not prompt for a
    // password it did not ask to need.
    struct Forbidden;

    impl PrivilegedReplace for Forbidden {
        fn replace(
            &self,
            _artifact: &VerifiedArtifact,
            _target: &Path,
        ) -> Result<(), InstallFailure> {
            panic!("escalation must not be attempted on a writable target");
        }
    }

    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();

    fixture
        .installer()
        .with_escalation(Forbidden)
        .apply(&verified(&fixture, bytes, &signature))
        .expect("a writable target needs no escalation");
}

#[test]
fn the_default_escalation_declines_rather_than_prompting() {
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();
    let artifact = verified(&fixture, bytes, &signature);
    assert!(matches!(
        NoPrivilegedReplace.replace(&artifact, Path::new("/target")),
        Err(InstallFailure::NotWritable { .. })
    ));
}

/// An escalated swap says which class it failed in, because the controller
/// acts on the difference: a malformed artifact is terminal and its staged
/// bytes are discarded, a partial replacement is retryable and names what it
/// kept.
#[cfg(unix)]
#[test]
fn escalation_failures_keep_their_contract_class() {
    use std::os::unix::fs::PermissionsExt;

    struct Reports(InstallFailure);

    impl PrivilegedReplace for Reports {
        fn replace(
            &self,
            _artifact: &VerifiedArtifact,
            _target: &Path,
        ) -> Result<(), InstallFailure> {
            Err(self.0.clone())
        }
    }

    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();
    let artifact = verified(&fixture, bytes, &signature);
    let malformed = InstallFailure::MalformedArtifact {
        detail: "protected extraction refused the archive".to_owned(),
    };
    let partial = InstallFailure::Failed {
        detail: "could not move the new bundle (permission denied). Rollback failed \
                 (permission denied). Previous bundle preserved at /protected/previous"
            .to_owned(),
    };

    // An unwritable install directory: the escalated path is the one taken.
    let parent = fixture.target.parent().unwrap();
    let permissions = fs::metadata(parent).unwrap().permissions();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();
    let malformed_outcome = fixture
        .installer()
        .with_escalation(Reports(malformed.clone()))
        .apply(&artifact);
    let partial_outcome = fixture
        .installer()
        .with_escalation(Reports(partial.clone()))
        .apply(&artifact);
    fs::set_permissions(parent, permissions).unwrap();

    assert_eq!(malformed_outcome, Err(malformed));
    assert_eq!(partial_outcome, Err(partial));
}

/// The escalation contract: the port receives the verified artifact, never a
/// user-writable staged tree, and an unwritable target reaches it.
#[cfg(unix)]
#[test]
fn escalation_receives_the_verified_artifact_not_a_staged_path() {
    use std::os::unix::fs::PermissionsExt;

    struct Spy;

    impl PrivilegedReplace for Spy {
        fn replace(
            &self,
            artifact: &VerifiedArtifact,
            _target: &Path,
        ) -> Result<(), InstallFailure> {
            // The implementor's side of the contract: extract the artifact
            // itself, into its own protected staging, via the same bounded
            // extraction the unprivileged path gets.
            let protected = TempDir::new().map_err(|error| InstallFailure::Failed {
                detail: error.to_string(),
            })?;
            let root = longhorn_update_install::extract_bundle(artifact, protected.path())?;
            let marker = fs::read_to_string(root.join("Contents/marker")).map_err(|error| {
                InstallFailure::Failed {
                    detail: error.to_string(),
                }
            })?;
            assert_eq!(marker, "new", "the artifact extracts under the port");
            assert_eq!(artifact.version(), &Version::parse("1.3.0").unwrap());
            Ok(())
        }
    }

    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();

    // An unwritable install directory: staging cannot be created, which is
    // exactly the case escalation exists for.
    let parent = fixture.target.parent().unwrap();
    let original = fs::metadata(parent).unwrap().permissions();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();
    let outcome = fixture
        .installer()
        .with_escalation(Spy)
        .apply(&verified(&fixture, bytes, &signature));
    fs::set_permissions(parent, original).unwrap();

    let applied = outcome.expect("the escalated path applies");
    assert_eq!(applied.version, fixture.fixtures.version());
    // The target was never writable, so the marker is unchanged — the spy
    // proved extraction, and a real implementor would also move.
    assert_eq!(fixture.marker(), "old");
}

#[cfg(unix)]
#[test]
fn an_unwritable_parent_serializes_escalation_without_a_workspace() {
    use std::{os::unix::fs::PermissionsExt, sync::mpsc, time::Duration};

    struct Paused(mpsc::Sender<()>, mpsc::Receiver<()>);

    impl PrivilegedReplace for Paused {
        fn replace(
            &self,
            _artifact: &VerifiedArtifact,
            _target: &Path,
        ) -> Result<(), InstallFailure> {
            self.0.send(()).map_err(|error| InstallFailure::Failed {
                detail: error.to_string(),
            })?;
            self.1
                .recv_timeout(Duration::from_secs(5))
                .map_err(|error| InstallFailure::Failed {
                    detail: error.to_string(),
                })
        }
    }

    struct AllowsReplacement;

    impl PrivilegedReplace for AllowsReplacement {
        fn replace(
            &self,
            _artifact: &VerifiedArtifact,
            _target: &Path,
        ) -> Result<(), InstallFailure> {
            Ok(())
        }
    }

    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();
    let artifact = verified(&fixture, bytes, &signature);
    let parent = fixture.parent();
    let permissions = fs::metadata(parent).unwrap().permissions();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();

    let (entered, entered_port) = mpsc::channel();
    let (resume, wait_for_resume) = mpsc::channel();
    let (first, overlapping) = std::thread::scope(|scope| {
        let target = &fixture.target;
        let artifact = &artifact;
        let first = scope.spawn(move || {
            NativeInstaller::new(target)
                .with_escalation(Paused(entered, wait_for_resume))
                .apply(artifact)
        });
        let overlapping = entered_port.recv_timeout(Duration::from_secs(5)).map(|()| {
            fixture
                .installer()
                .with_escalation(AllowsReplacement)
                .apply(artifact)
        });
        let _ = resume.send(());
        (first.join(), overlapping)
    });
    fs::set_permissions(parent, permissions).unwrap();

    first.unwrap().expect("the first escalation completes");
    let error = overlapping
        .expect("the first apply entered the port")
        .expect_err("the overlapping apply must stay out of the port");
    assert!(error.to_string().contains("another update is active"));
    assert!(!fixture.workspace().exists());
}

#[test]
fn a_failed_install_leaves_no_staging_directory_behind() {
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.signed_but_unusable().unwrap();

    fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .expect_err("malformed artifact");

    for entry in fs::read_dir(fixture.workspace()).unwrap() {
        assert!(entry.unwrap().file_type().unwrap().is_file());
    }
}

#[test]
fn relaunch_is_left_to_the_host() {
    // macOS separates replacement from relaunch. Longhorn keeps that
    // separation rather than hiding it, so the caller orders teardown.
    let fixture = Fixture::new();
    let (bytes, signature) = fixture.fixtures.valid();

    let applied = fixture
        .installer()
        .apply(&verified(&fixture, bytes, &signature))
        .unwrap();

    assert!(!applied.relaunched);
}

/// Verification now happens before an installer is reachable, so every test
/// that used to hand bytes straight to `apply` has to pass through it. A
/// fixture whose signature does not verify fails here, loudly, rather than
/// inside the assertion it was setting up.
fn verified(fixture: &Fixture, bytes: Vec<u8>, signature: &str) -> VerifiedArtifact {
    verify_artifact(
        &fixture.fixtures.key(),
        &fixture.fixtures.version(),
        bytes,
        signature,
    )
    .expect("fixture verifies")
}
