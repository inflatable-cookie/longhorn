//! Evidence that the installer conformance suite catches what it claims to.
//!
//! A suite nobody has seen fail is a suite nobody should trust. Each test
//! here feeds it a deliberately wrong implementation and asserts the specific
//! claim goes red.
//!
//! # What Card 196 removed, and why nothing is weaker
//!
//! There used to be an `Unverifying` case: an installer that applied whatever
//! it was handed, proving the suite caught an implementation that skipped
//! verification. It cannot be written any more. `apply` takes a
//! `VerifiedArtifact`, which only `verify_artifact` constructs, so an
//! implementation has nothing to skip.
//!
//! The tampered claim stays in the suite and now exercises the shared
//! verifier against the fixture's own key. That is still per-implementation
//! and still worth running — it proves the signing that produced the fixtures
//! agrees with the verification the controller will do — but it is no longer
//! a claim about the installer's diligence, because the installer no longer
//! has any in this area.

use longhorn_update::{
    Applied, ArtifactKey, ConformanceFixtures, InstallFailure, UpdateInstaller, VerifiedArtifact,
    run_conformance, verify_artifact,
};
use minisign::KeyPair;
use semver::Version;
use std::io::Cursor;

const VALID: &[u8] = b"a valid bundle";
const TAMPERED: &[u8] = b"a tampered bundle";
const UNUSABLE: &[u8] = b"signed but not a bundle";

/// Real minisign material. The fixtures used to carry the string
/// `"good-signature"`, which was enough while each installer decided for
/// itself what verification meant and is not now that the suite verifies.
struct Fixtures {
    keys: KeyPair,
}

impl Fixtures {
    fn new() -> Self {
        Self {
            keys: KeyPair::generate_unencrypted_keypair().expect("keypair"),
        }
    }

    fn sign(&self, bytes: &[u8]) -> String {
        minisign::sign(None, &self.keys.sk, Cursor::new(bytes), None, None)
            .expect("signs")
            .to_string()
    }

    /// A signature over `VALID`, which does not match any other bytes.
    fn valid_signature(&self) -> String {
        self.sign(VALID)
    }
}

impl ConformanceFixtures for Fixtures {
    fn version(&self) -> Version {
        Version::parse("1.3.0").unwrap()
    }

    fn key(&self) -> ArtifactKey {
        ArtifactKey::from_base64(&self.keys.pk.to_base64()).expect("public key")
    }

    fn valid(&self) -> (Vec<u8>, String) {
        (VALID.to_vec(), self.valid_signature())
    }

    fn tampered(&self) -> (Vec<u8>, String) {
        // The valid signature against different bytes: exactly what an
        // attacker who cannot sign but can substitute has to offer.
        (TAMPERED.to_vec(), self.valid_signature())
    }

    fn signed_but_unusable(&self) -> Option<(Vec<u8>, String)> {
        Some((UNUSABLE.to_vec(), self.sign(UNUSABLE)))
    }
}

/// A correct installer: applies what it is given, and reports what it cannot
/// use as unusable rather than as untrusted.
struct Correct;

impl UpdateInstaller for Correct {
    fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
        if artifact.bytes() == UNUSABLE {
            return Err(InstallFailure::MalformedArtifact {
                detail: "not an application bundle".into(),
            });
        }
        Ok(Applied {
            version: artifact.version().clone(),
            relaunched: true,
        })
    }
}

fn claim<'a>(
    outcomes: &'a [longhorn_update::ConformanceOutcome],
    needle: &str,
) -> &'a longhorn_update::ConformanceOutcome {
    outcomes
        .iter()
        .find(|outcome| outcome.claim.contains(needle))
        .expect("claim is part of the suite")
}

#[test]
fn a_correct_installer_satisfies_every_claim() {
    let outcomes = run_conformance(&Correct, &Fixtures::new());

    assert_eq!(outcomes.len(), 4);
    for outcome in &outcomes {
        assert!(outcome.satisfied, "{}: {:?}", outcome.claim, outcome.detail);
    }
}

/// The claim that used to be about the installer and is now about the
/// verifier. A substituted artifact carrying a genuine signature for
/// different bytes must not verify.
#[test]
fn a_tampered_artifact_never_becomes_a_verified_one() {
    let fixtures = Fixtures::new();
    let (bytes, signature) = fixtures.tampered();

    assert_eq!(
        verify_artifact(&fixtures.key(), &fixtures.version(), bytes, &signature),
        Err(InstallFailure::SignatureRejected)
    );
}

/// A signature from a different keypair, which is the other half of the same
/// claim: right shape, wrong signer.
#[test]
fn an_artifact_signed_by_another_key_never_becomes_a_verified_one() {
    let ours = Fixtures::new();
    let theirs = Fixtures::new();
    let (bytes, signature) = theirs.valid();

    assert_eq!(
        verify_artifact(&ours.key(), &ours.version(), bytes, &signature),
        Err(InstallFailure::SignatureRejected)
    );
}

/// Not a signature at all. Decoding failure and verification failure are the
/// same event to a caller, and must not be two vocabularies.
#[test]
fn an_unparseable_signature_is_a_signature_rejection() {
    let fixtures = Fixtures::new();

    assert_eq!(
        verify_artifact(
            &fixtures.key(),
            &fixtures.version(),
            VALID.to_vec(),
            "not a signature",
        ),
        Err(InstallFailure::SignatureRejected)
    );
}

/// The verified artifact carries the version the caller asked for, not one
/// read out of the bytes. Unless the signature's trusted comment binds one —
/// then the binding must match, which the next test proves.
#[test]
fn a_verified_artifact_carries_the_requested_version() {
    let fixtures = Fixtures::new();
    let (bytes, signature) = fixtures.valid();
    let verified =
        verify_artifact(&fixtures.key(), &fixtures.version(), bytes, &signature).expect("verifies");

    assert_eq!(verified.version(), &fixtures.version());
    assert_eq!(verified.bytes(), VALID);
}

/// The downgrade defence, enforced when present: a trusted comment of
/// `version:<semver>` is bound to the artifact by minisign's global
/// signature, so it must equal the version being fetched.
#[test]
fn a_version_bound_signature_must_match_the_requested_version() {
    let fixtures = Fixtures::new();
    let sign_bound = |comment: &str| {
        minisign::sign(
            None,
            &fixtures.keys.sk,
            Cursor::new(VALID),
            Some(comment),
            None,
        )
        .expect("signs")
        .to_string()
    };

    // Matching binds verify.
    let matching = sign_bound("version:1.3.0");
    verify_artifact(
        &fixtures.key(),
        &fixtures.version(),
        VALID.to_vec(),
        &matching,
    )
    .expect("a signature bound to this version verifies");

    // A genuine signature over the right bytes, bound to an older version,
    // is the downgrade: refused as a signature failure, not applied.
    let downgraded = sign_bound("version:1.2.0");
    assert_eq!(
        verify_artifact(
            &fixtures.key(),
            &fixtures.version(),
            VALID.to_vec(),
            &downgraded
        ),
        Err(InstallFailure::SignatureRejected)
    );

    // A bound that is not a version at all is malformed, not ignored.
    let nonsense = sign_bound("version:not-a-version");
    assert_eq!(
        verify_artifact(
            &fixtures.key(),
            &fixtures.version(),
            VALID.to_vec(),
            &nonsense
        ),
        Err(InstallFailure::SignatureRejected)
    );

    // And a signature without a version comment — Tauri's signing emits a
    // timestamp — verifies as before. The residual is recorded in the
    // verifier's doc and contract 018.
    let unbound = fixtures.valid_signature();
    verify_artifact(
        &fixtures.key(),
        &fixtures.version(),
        VALID.to_vec(),
        &unbound,
    )
    .expect("an unbound signature still verifies");
}

#[test]
fn conflating_an_unusable_artifact_with_a_signature_failure_fails_the_suite() {
    // "We do not trust this" and "we trust it and cannot use it" are
    // different messages, and only one is a security event.
    struct Conflating;

    impl UpdateInstaller for Conflating {
        fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
            if artifact.bytes() == VALID {
                return Ok(Applied {
                    version: artifact.version().clone(),
                    relaunched: true,
                });
            }
            Err(InstallFailure::SignatureRejected)
        }
    }

    let outcomes = run_conformance(&Conflating, &Fixtures::new());

    assert!(claim(&outcomes, "tampered").satisfied);
    assert!(!claim(&outcomes, "unusable").satisfied);
}

#[test]
fn applying_the_wrong_version_fails_the_suite() {
    struct WrongVersion;

    impl UpdateInstaller for WrongVersion {
        fn apply(&self, _artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
            Ok(Applied {
                version: Version::parse("9.9.9").unwrap(),
                relaunched: true,
            })
        }
    }

    let outcomes = run_conformance(&WrongVersion, &Fixtures::new());

    assert!(!claim(&outcomes, "applied version").satisfied);
}

#[test]
fn the_suite_reports_every_failure_rather_than_the_first() {
    // A report naming one fault invites fixing it and rerunning blind.
    struct WhollyWrong;

    impl UpdateInstaller for WhollyWrong {
        fn apply(&self, _artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
            Err(InstallFailure::Failed {
                detail: "nothing works".into(),
            })
        }
    }

    let outcomes = run_conformance(&WhollyWrong, &Fixtures::new());
    let failures = outcomes.iter().filter(|o| !o.satisfied).count();

    assert!(failures >= 3, "expected several failures, found {failures}");
}

#[test]
fn failure_classification_drives_the_right_response() {
    assert!(
        InstallFailure::Failed {
            detail: "network".into()
        }
        .is_retryable()
    );
    assert!(
        !InstallFailure::SignatureRejected.is_retryable(),
        "a rejected signature must never be retried"
    );
    assert!(
        InstallFailure::NotWritable {
            detail: "/Applications/Example.app".into()
        }
        .needs_manual_action()
    );
    assert!(
        !InstallFailure::Failed {
            detail: "network".into()
        }
        .needs_manual_action()
    );
}
