//! Contract 027: digest sources, artifact verification and the signature
//! port, through the public API only.

mod support;

use std::path::Path;

use longhorn_tool_provision::{
    CodesignVerifier, Declaration, DigestOrigin, DigestSource, Fault, ProvisionError, Refusal,
    SignatureReport, SignatureVerifier, digest_from_checksums, digest_from_sidecar, sha256_file,
    verify_artifact, verify_signature,
};
use support::write_file;

/// SHA-256 of the bytes `bovine`; `printf 'bovine' | shasum -a 256`.
const BOVINE: &[u8] = b"bovine";
const BOVINE_DIGEST: &str = "f6ed748ef16bc1b2334123f4563472e13b0e930015814413ef2780bee0071687";

/// A verifier that answers from a script, so the signature port is exercised
/// without a vendor-signed binary.
struct Scripted(Result<SignatureReport, ProvisionError>);

impl SignatureVerifier for Scripted {
    fn report(&self, _binary: &Path) -> Result<SignatureReport, ProvisionError> {
        self.0.clone()
    }
}

/// A verifier that fails the test if it is consulted.
struct NeverConsulted;

impl SignatureVerifier for NeverConsulted {
    fn report(&self, _binary: &Path) -> Result<SignatureReport, ProvisionError> {
        panic!("a digest-only declaration must not consult the signature verifier");
    }
}

fn team(name: &str) -> SignatureReport {
    SignatureReport::Team(name.to_owned())
}

fn refused(error: ProvisionError) -> Refusal {
    match error {
        ProvisionError::Refused(refusal) => refusal,
        ProvisionError::Fault(fault) => panic!("expected a refusal, got fault {fault}"),
    }
}

#[test]
fn checksum_entries_parse_and_match_names() {
    let amd64 = "a".repeat(64);
    let arm64 = "B".repeat(64);
    let checksums = format!(
        "{amd64}  tool_2.0.0_macOS_amd64.zip\n{arm64} *tool_2.0.0_macOS_arm64.zip\n\n{}  other.txt\n",
        "c".repeat(64)
    );
    assert_eq!(
        digest_from_checksums(&checksums, "tool_2.0.0_macOS_arm64.zip").unwrap(),
        arm64.to_ascii_lowercase(),
        "the binary-marker form parses and the digest normalises to lowercase"
    );
    assert_eq!(
        digest_from_checksums(&checksums, "tool_2.0.0_macOS_amd64.zip").unwrap(),
        amd64
    );
    assert!(matches!(
        refused(digest_from_checksums(&checksums, "absent.zip").unwrap_err()),
        Refusal::ChecksumNotCovered { .. }
    ));
}

#[test]
fn malformed_and_ambiguous_checksum_entries_refuse() {
    for checksums in [
        "nothex  artifact.zip\n".to_owned(),
        "abcd  artifact.zip\n".to_owned(),
        // The same asset listed twice: two digests, neither trusted.
        format!(
            "{}  artifact.zip\n{}  artifact.zip\n",
            "a".repeat(64),
            "b".repeat(64)
        ),
    ] {
        assert!(
            matches!(
                refused(digest_from_checksums(&checksums, "artifact.zip").unwrap_err()),
                Refusal::MalformedChecksum { .. }
            ),
            "{checksums:?}"
        );
    }
}

#[test]
fn sidecars_accept_only_the_canonical_line() {
    let digest = "a".repeat(64);
    let sidecar = format!("{digest}  tool-darwin-arm64\n");
    assert_eq!(
        digest_from_sidecar(&sidecar, "tool-darwin-arm64").unwrap(),
        digest
    );
    for (sidecar, asset) in [
        (sidecar.clone(), "tool-darwin-x64"),
        (
            format!("{}  tool-darwin-arm64\n", "A".repeat(64)),
            "tool-darwin-arm64",
        ),
        (format!("{sidecar}extra\n"), "tool-darwin-arm64"),
        (format!("{digest}  tool-darwin-arm64"), "tool-darwin-arm64"),
    ] {
        assert!(
            matches!(
                refused(digest_from_sidecar(&sidecar, asset).unwrap_err()),
                Refusal::MalformedSidecar { .. }
            ),
            "{sidecar:?} for {asset}"
        );
    }
}

#[test]
fn a_digest_mismatch_refuses_with_its_origin() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "artifact", BOVINE);
    let checksums = format!("{BOVINE_DIGEST}  artifact\n");
    verify_artifact(
        &path,
        "artifact",
        Declaration {
            digest: Some(DigestSource::ReleaseChecksums(&checksums)),
            vendor_team: None,
        },
    )
    .expect("the digest matches");

    let wrong = format!("{}  artifact\n", "0".repeat(64));
    assert_eq!(
        refused(
            verify_artifact(
                &path,
                "artifact",
                Declaration {
                    digest: Some(DigestSource::ReleaseChecksums(&wrong)),
                    vendor_team: None,
                },
            )
            .unwrap_err()
        ),
        Refusal::DigestMismatch {
            asset: "artifact".to_owned(),
            origin: DigestOrigin::ReleaseChecksums,
        }
    );
}

#[test]
fn a_pinned_digest_verifies_exact_bytes_and_refuses_tampering() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "tool-darwin-arm64", BOVINE);
    let declared = Declaration {
        digest: Some(DigestSource::Pinned(BOVINE_DIGEST)),
        vendor_team: None,
    };
    verify_artifact(&path, "tool-darwin-arm64", declared).expect("the pinned bytes verify");

    std::fs::write(&path, b"tampered").unwrap();
    assert_eq!(
        refused(verify_artifact(&path, "tool-darwin-arm64", declared).unwrap_err()),
        Refusal::DigestMismatch {
            asset: "tool-darwin-arm64".to_owned(),
            origin: DigestOrigin::Pinned,
        }
    );
    assert!(matches!(
        refused(
            verify_artifact(
                &path,
                "tool-darwin-arm64",
                Declaration {
                    digest: Some(DigestSource::Pinned(&BOVINE_DIGEST.to_uppercase())),
                    vendor_team: None,
                },
            )
            .unwrap_err()
        ),
        Refusal::MalformedPin { .. }
    ));
}

#[test]
fn neither_source_declared_refuses_before_the_file_is_read() {
    // The path does not exist: a refusal here proves nothing was hashed.
    let missing = Path::new("/nonexistent/never-written-artifact");
    assert!(matches!(
        refused(
            verify_artifact(
                missing,
                "artifact",
                Declaration {
                    digest: None,
                    vendor_team: None,
                },
            )
            .unwrap_err()
        ),
        Refusal::NothingDeclared { .. }
    ));
}

#[test]
fn a_vendor_team_alone_is_a_declaration_and_still_needs_a_signature_later() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "artifact", BOVINE);
    assert!(
        verify_artifact(
            &path,
            "artifact",
            Declaration {
                digest: None,
                vendor_team: Some("TEAMID0001"),
            },
        )
        .is_ok()
    );
}

#[test]
fn an_unreadable_artifact_is_a_fault_not_a_refusal() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("absent");
    let checksums = format!("{BOVINE_DIGEST}  absent\n");
    let error = verify_artifact(
        &missing,
        "absent",
        Declaration {
            digest: Some(DigestSource::ReleaseChecksums(&checksums)),
            vendor_team: None,
        },
    )
    .unwrap_err();
    assert!(
        matches!(error, ProvisionError::Fault(Fault::Io { .. })),
        "{error}"
    );
    assert_eq!(
        sha256_file(&temp.path().join("absent")).unwrap_err(),
        error,
        "sha256_file reports the same fault"
    );
}

#[test]
fn a_digest_only_declaration_never_consults_the_signature_verifier() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "tool", BOVINE);
    let verified = verify_artifact(
        &path,
        "tool",
        Declaration {
            digest: Some(DigestSource::Pinned(BOVINE_DIGEST)),
            vendor_team: None,
        },
    )
    .unwrap();
    // Extraction is covered in extract.rs; here the staged file is the
    // artifact's own bytes, so the signature step is exercised directly.
    let staged = longhorn_tool_provision::extract_executable(
        &verified,
        longhorn_tool_provision::ArtifactShape::Raw,
        temp.path(),
        "tool-staged",
        longhorn_tool_provision::ExtractionLimits::DEFAULT,
    )
    .unwrap();
    let installable =
        verify_signature(staged, &NeverConsulted).expect("no team, no signature check");
    assert_eq!(installable.vendor_team(), None);
}

#[test]
fn the_signature_check_matches_the_declared_team_only() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "tool", BOVINE);
    let staged_for = |name: &str| {
        let verified = verify_artifact(
            &path,
            "tool",
            Declaration {
                digest: None,
                vendor_team: Some("TEAMID0001"),
            },
        )
        .unwrap();
        longhorn_tool_provision::extract_executable(
            &verified,
            longhorn_tool_provision::ArtifactShape::Raw,
            temp.path(),
            name,
            longhorn_tool_provision::ExtractionLimits::DEFAULT,
        )
        .unwrap()
    };

    let installable = verify_signature(staged_for("matching"), &Scripted(Ok(team("TEAMID0001"))))
        .expect("the declared team verifies");
    assert_eq!(installable.vendor_team(), Some("TEAMID0001"));
    assert!(installable.path().is_file());

    assert!(matches!(
        refused(
            verify_signature(staged_for("other"), &Scripted(Ok(team("OTHERTEAM1"))))
                .unwrap_err()
        ),
        Refusal::TeamMismatch { found, expected } if found == "OTHERTEAM1" && expected == "TEAMID0001"
    ));

    assert!(matches!(
        refused(
            verify_signature(
                staged_for("unsigned"),
                &Scripted(Ok(SignatureReport::Unverified {
                    detail: "no signature".to_owned(),
                })),
            )
            .unwrap_err()
        ),
        Refusal::Unsigned { expected, .. } if expected == "TEAMID0001"
    ));
}

#[test]
fn a_verifier_that_cannot_run_is_a_fault_not_a_judgment() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "tool", BOVINE);
    let verified = verify_artifact(
        &path,
        "tool",
        Declaration {
            digest: None,
            vendor_team: Some("TEAMID0001"),
        },
    )
    .unwrap();
    let staged = longhorn_tool_provision::extract_executable(
        &verified,
        longhorn_tool_provision::ArtifactShape::Raw,
        temp.path(),
        "tool-staged",
        longhorn_tool_provision::ExtractionLimits::DEFAULT,
    )
    .unwrap();
    let error = verify_signature(
        staged,
        &Scripted(Err(ProvisionError::Fault(Fault::Verifier {
            detail: "codesign could not run".to_owned(),
        }))),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProvisionError::Fault(Fault::Verifier { .. })
    ));
}

#[test]
#[cfg(target_os = "macos")]
fn the_codesign_adapter_refuses_an_unsigned_binary() {
    // No vendor-signed binary is needed: an unsigned file must be refused by
    // the real adapter, which runs the fixed /usr/bin/codesign path.
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "tool", BOVINE);
    let verified = verify_artifact(
        &path,
        "tool",
        Declaration {
            digest: None,
            vendor_team: Some("TEAMID0001"),
        },
    )
    .unwrap();
    let staged = longhorn_tool_provision::extract_executable(
        &verified,
        longhorn_tool_provision::ArtifactShape::Raw,
        temp.path(),
        "tool-staged",
        longhorn_tool_provision::ExtractionLimits::DEFAULT,
    )
    .unwrap();
    assert!(matches!(
        refused(verify_signature(staged, &CodesignVerifier).unwrap_err()),
        Refusal::Unsigned { .. }
    ));
}

#[test]
#[cfg(not(target_os = "macos"))]
fn the_codesign_adapter_refuses_a_declared_team_off_macos() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_file(temp.path(), "tool", BOVINE);
    let verified = verify_artifact(
        &path,
        "tool",
        Declaration {
            digest: None,
            vendor_team: Some("TEAMID0001"),
        },
    )
    .unwrap();
    let staged = longhorn_tool_provision::extract_executable(
        &verified,
        longhorn_tool_provision::ArtifactShape::Raw,
        temp.path(),
        "tool-staged",
        longhorn_tool_provision::ExtractionLimits::DEFAULT,
    )
    .unwrap();
    assert!(matches!(
        refused(verify_signature(staged, &CodesignVerifier).unwrap_err()),
        Refusal::SignatureUnsupported { .. }
    ));
}
