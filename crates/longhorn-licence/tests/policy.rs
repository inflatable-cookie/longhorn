//! Trust basis, lease and grace, clock regression, and verification.

use ed25519_dalek::{Signer, SigningKey};
use longhorn_licence::{
    ClockGuard, GracePolicy, LicencePayload, SignedLicence, Span, Timestamp, TrustBasis, Usability,
    VerificationError, VerifiedLicence, usability, verify,
};

const DAY: i64 = 86_400;

fn at(day: i64) -> Timestamp {
    Timestamp::from_unix_seconds(day * DAY)
}

fn signing_key() -> SigningKey {
    // Fixed bytes: the test needs a key, not entropy.
    SigningKey::from_bytes(&[7_u8; 32])
}

fn sign(payload: &LicencePayload) -> SignedLicence {
    let key = signing_key();
    let bytes = serde_json::to_vec(payload).unwrap();
    let signature = key.sign(&bytes);
    SignedLicence::new("test-key", bytes, signature.to_bytes().to_vec())
}

fn leased(until: i64) -> LicencePayload {
    LicencePayload::new("example").with_lease_until(at(until))
}

// -- trust basis ------------------------------------------------------------

#[test]
fn a_signature_is_offline_verifiable_and_a_remote_assertion_is_not() {
    let signed = verify(&sign(&leased(30)), &signing_key().verifying_key()).unwrap();
    let remote = VerifiedLicence::from_remote_assertion(leased(30), at(0));

    assert!(signed.basis().is_offline_verifiable());
    assert!(!remote.basis().is_offline_verifiable());
    assert!(matches!(
        signed.basis(),
        TrustBasis::OfflineSignature { key_id } if key_id == "test-key"
    ));
}

#[test]
fn a_remote_assertion_gets_less_grace_than_a_signature() {
    // The central rule: offline grace is never granted on a basis incapable
    // of surviving being offline. At day 40 the signature licence is still
    // in grace and the remote one has lapsed, from identical payloads.
    let grace = GracePolicy::new(Span::from_days(30), Span::from_days(7));
    let guard = ClockGuard::new(at(0));

    let signed = verify(&sign(&leased(30)), &signing_key().verifying_key()).unwrap();
    let remote = VerifiedLicence::from_remote_assertion(leased(30), at(0));

    assert_eq!(
        usability(&signed, at(40), guard, grace),
        Usability::InGrace { until: at(60) }
    );
    assert_eq!(
        usability(&remote, at(40), guard, grace),
        Usability::LeaseLapsed { at: at(37) }
    );
}

#[test]
fn remote_grace_cannot_be_configured_above_signature_grace() {
    // The clamp is an invariant, not a convenience. A consumer cannot hand
    // a remote assertion more tolerance than a signature by configuration.
    let grace = GracePolicy::new(Span::from_days(7), Span::from_days(90));

    assert_eq!(
        grace.for_basis(&TrustBasis::RemoteAssertion { checked: at(0) }),
        Span::from_days(7)
    );
}

// -- lease, grace, fail-open ------------------------------------------------

#[test]
fn an_unreachable_backend_within_the_lease_changes_nothing() {
    // Fail open. Nothing here consults reachability at all: an outage is
    // simply a lease that has not been renewed yet, and inside the lease
    // that is indistinguishable from a healthy licence.
    let licence = VerifiedLicence::from_remote_assertion(leased(30), at(0));

    assert_eq!(
        usability(
            &licence,
            at(29),
            ClockGuard::new(at(0)),
            GracePolicy::default()
        ),
        Usability::Active
    );
}

#[test]
fn grace_is_usable_and_does_not_warrant_attention() {
    let licence = verify(&sign(&leased(30)), &signing_key().verifying_key()).unwrap();

    let state = usability(
        &licence,
        at(40),
        ClockGuard::new(at(0)),
        GracePolicy::default(),
    );

    assert!(state.is_usable());
    assert!(
        !state.warrants_attention(),
        "a renewal inside its tolerance is not something the user can act on"
    );
}

#[test]
fn a_licence_with_no_lease_never_needs_revalidating() {
    let perpetual = verify(
        &sign(&LicencePayload::new("example")),
        &signing_key().verifying_key(),
    )
    .unwrap();

    assert_eq!(
        usability(
            &perpetual,
            at(100_000),
            ClockGuard::new(at(0)),
            GracePolicy::default()
        ),
        Usability::Active
    );
}

#[test]
fn an_expired_use_window_outranks_a_lapsed_lease() {
    // Ordering matters for the message: "your subscription ended" is truer
    // and more actionable than "revalidation failed".
    let licence = VerifiedLicence::from_remote_assertion(
        LicencePayload::new("example")
            .with_use_until(at(30))
            .with_lease_until(at(10)),
        at(0),
    );

    assert_eq!(
        usability(
            &licence,
            at(100),
            ClockGuard::new(at(0)),
            GracePolicy::default()
        ),
        Usability::UseWindowExpired { at: at(30) }
    );
}

// -- clock ------------------------------------------------------------------

#[test]
fn a_large_backwards_clock_movement_is_refused() {
    let licence = VerifiedLicence::from_remote_assertion(leased(30), at(0));
    let guard = ClockGuard::new(at(100));

    assert_eq!(
        usability(&licence, at(5), guard, GracePolicy::default()),
        Usability::ClockRefused
    );
}

#[test]
fn a_small_backwards_movement_is_tolerated() {
    // An NTP correction, a timezone mistake, or a dead CMOS battery is not
    // abuse, and refusing it would punish the wrong person.
    let licence = VerifiedLicence::from_remote_assertion(leased(30), at(0));
    let guard = ClockGuard::new(Timestamp::from_unix_seconds(10 * DAY));

    let slightly_behind = Timestamp::from_unix_seconds(10 * DAY - 3_600);

    assert_eq!(
        usability(&licence, slightly_behind, guard, GracePolicy::default()),
        Usability::Active
    );
}

#[test]
fn the_guard_advances_with_the_clock_but_never_retreats() {
    let guard = ClockGuard::new(at(10));

    assert_eq!(guard.observing(at(20)).highest_seen, at(20));
    assert_eq!(guard.observing(at(5)).highest_seen, at(10));
}

#[test]
fn a_zero_tolerance_guard_refuses_any_regression() {
    let licence = VerifiedLicence::from_remote_assertion(leased(30), at(0));
    let guard = ClockGuard::new(at(10)).with_tolerance(Span::ZERO);

    assert_eq!(
        usability(
            &licence,
            Timestamp::from_unix_seconds(10 * DAY - 1),
            guard,
            GracePolicy::default()
        ),
        Usability::ClockRefused
    );
}

// -- verification -----------------------------------------------------------

#[test]
fn a_tampered_payload_fails_verification() {
    let mut signed = sign(&leased(30));
    // Flip a byte inside the signed bytes. The signature no longer covers
    // them, which is the whole point of signing bytes rather than a parse.
    let last = signed.payload.len() - 1;
    signed.payload[last] ^= 0x01;

    assert_eq!(
        verify(&signed, &signing_key().verifying_key()),
        Err(VerificationError::SignatureRejected)
    );
}

#[test]
fn a_signature_from_another_key_is_rejected() {
    let signed = sign(&leased(30));
    let other = SigningKey::from_bytes(&[9_u8; 32]);

    assert_eq!(
        verify(&signed, &other.verifying_key()),
        Err(VerificationError::SignatureRejected)
    );
}

#[test]
fn a_malformed_signature_is_reported_as_such() {
    let mut signed = sign(&leased(30));
    signed.signature.truncate(10);

    assert!(matches!(
        verify(&signed, &signing_key().verifying_key()),
        Err(VerificationError::MalformedSignature { actual: 10, .. })
    ));
}

#[test]
fn the_verified_payload_is_the_bytes_that_were_signed() {
    // Whitespace that a re-serialisation would normalise away. Verification
    // covers the received bytes, so this must verify and parse identically
    // -- a scheme that re-serialised before verifying would reject it, and
    // one that verified a re-serialisation could be forged through it.
    let key = signing_key();
    let payload = leased(30);
    let spaced = format!("{}   ", serde_json::to_string(&payload).unwrap());
    let bytes = spaced.into_bytes();
    let signature = key.sign(&bytes);
    let signed = SignedLicence::new("test-key", bytes, signature.to_bytes().to_vec());

    let verified = verify(&signed, &key.verifying_key()).unwrap();

    assert_eq!(verified.payload(), &payload);
}

#[test]
fn signed_bytes_that_are_not_a_licence_are_reported_after_verification() {
    let key = signing_key();
    let bytes = b"not a licence".to_vec();
    let signature = key.sign(&bytes);
    let signed = SignedLicence::new("test-key", bytes, signature.to_bytes().to_vec());

    assert!(matches!(
        verify(&signed, &key.verifying_key()),
        Err(VerificationError::MalformedPayload { .. })
    ));
}
