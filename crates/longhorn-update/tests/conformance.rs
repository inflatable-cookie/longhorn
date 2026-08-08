//! Evidence that the installer conformance suite catches what it claims to.
//!
//! A suite nobody has seen fail is a suite nobody should trust. Each test
//! here feeds it a deliberately wrong implementation and asserts the
//! specific claim goes red.

use longhorn_update::{
    Applied, ConformanceFixtures, InstallFailure, UpdateInstaller, run_conformance,
};
use semver::Version;

const VALID: &[u8] = b"a valid bundle";
const TAMPERED: &[u8] = b"a tampered bundle";
const UNUSABLE: &[u8] = b"signed but not a bundle";

struct Fixtures;

impl ConformanceFixtures for Fixtures {
    fn version(&self) -> Version {
        Version::parse("1.3.0").unwrap()
    }

    fn valid(&self) -> (Vec<u8>, String) {
        (VALID.to_vec(), "good-signature".to_owned())
    }

    fn tampered(&self) -> (Vec<u8>, String) {
        (TAMPERED.to_vec(), "good-signature".to_owned())
    }

    fn signed_but_unusable(&self) -> Option<(Vec<u8>, String)> {
        Some((UNUSABLE.to_vec(), "good-signature".to_owned()))
    }
}

/// A correct installer: verifies first, applies only what verifies.
struct Correct;

impl UpdateInstaller for Correct {
    fn apply(
        &self,
        version: &Version,
        artifact: &[u8],
        _signature: &str,
    ) -> Result<Applied, InstallFailure> {
        if artifact == TAMPERED {
            return Err(InstallFailure::SignatureRejected);
        }
        if artifact == UNUSABLE {
            return Err(InstallFailure::MalformedArtifact {
                detail: "not an application bundle".into(),
            });
        }
        Ok(Applied {
            version: version.clone(),
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
    let outcomes = run_conformance(&Correct, &Fixtures);

    assert_eq!(outcomes.len(), 4);
    for outcome in &outcomes {
        assert!(outcome.satisfied, "{}: {:?}", outcome.claim, outcome.detail);
    }
}

#[test]
fn an_installer_that_applies_a_tampered_artifact_fails_the_suite() {
    // The claim that matters most. An implementation that skips verification
    // must not be able to pass.
    struct Unverifying;

    impl UpdateInstaller for Unverifying {
        fn apply(
            &self,
            version: &Version,
            _artifact: &[u8],
            _signature: &str,
        ) -> Result<Applied, InstallFailure> {
            Ok(Applied {
                version: version.clone(),
                relaunched: true,
            })
        }
    }

    let outcomes = run_conformance(&Unverifying, &Fixtures);
    let tampered = claim(&outcomes, "tampered");

    assert!(!tampered.satisfied);
    assert_eq!(tampered.detail.as_deref(), Some("the artifact was applied"));
}

#[test]
fn rejecting_a_tampered_artifact_for_the_wrong_reason_fails_the_suite() {
    // Refusing is not enough. A signature failure reported as a generic
    // fault invites a retry loop against an attacker-supplied artifact.
    struct WrongReason;

    impl UpdateInstaller for WrongReason {
        fn apply(
            &self,
            version: &Version,
            artifact: &[u8],
            _signature: &str,
        ) -> Result<Applied, InstallFailure> {
            if artifact == VALID {
                return Ok(Applied {
                    version: version.clone(),
                    relaunched: true,
                });
            }
            Err(InstallFailure::Failed {
                detail: "something went wrong".into(),
            })
        }
    }

    let outcomes = run_conformance(&WrongReason, &Fixtures);
    let tampered = claim(&outcomes, "tampered");

    assert!(!tampered.satisfied);
    assert!(
        tampered
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("rather than a signature failure"))
    );
}

#[test]
fn conflating_an_unusable_artifact_with_a_signature_failure_fails_the_suite() {
    // "We do not trust this" and "we trust it and cannot use it" are
    // different messages, and only one is a security event.
    struct Conflating;

    impl UpdateInstaller for Conflating {
        fn apply(
            &self,
            version: &Version,
            artifact: &[u8],
            _signature: &str,
        ) -> Result<Applied, InstallFailure> {
            if artifact == VALID {
                return Ok(Applied {
                    version: version.clone(),
                    relaunched: true,
                });
            }
            Err(InstallFailure::SignatureRejected)
        }
    }

    let outcomes = run_conformance(&Conflating, &Fixtures);

    assert!(claim(&outcomes, "tampered").satisfied);
    assert!(!claim(&outcomes, "unusable").satisfied);
}

#[test]
fn applying_the_wrong_version_fails_the_suite() {
    struct WrongVersion;

    impl UpdateInstaller for WrongVersion {
        fn apply(
            &self,
            _version: &Version,
            artifact: &[u8],
            _signature: &str,
        ) -> Result<Applied, InstallFailure> {
            if artifact != VALID {
                return Err(InstallFailure::SignatureRejected);
            }
            Ok(Applied {
                version: Version::parse("9.9.9").unwrap(),
                relaunched: true,
            })
        }
    }

    let outcomes = run_conformance(&WrongVersion, &Fixtures);

    assert!(!claim(&outcomes, "applied version").satisfied);
}

#[test]
fn the_suite_reports_every_failure_rather_than_the_first() {
    // A report naming one fault invites fixing it and rerunning blind.
    struct WhollyWrong;

    impl UpdateInstaller for WhollyWrong {
        fn apply(
            &self,
            _version: &Version,
            _artifact: &[u8],
            _signature: &str,
        ) -> Result<Applied, InstallFailure> {
            Err(InstallFailure::Failed {
                detail: "nothing works".into(),
            })
        }
    }

    let outcomes = run_conformance(&WhollyWrong, &Fixtures);
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
    assert!(!InstallFailure::SignatureRejected.needs_manual_action());
}
