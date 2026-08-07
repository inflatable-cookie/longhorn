//! Activation adapter acceptance evidence.

use ed25519_dalek::{Signer, SigningKey};
use longhorn_licence::{
    Activation, ActivationError, ActivationSource, ActivationUrl, ActivationUrlError, ClockGuard,
    Credential, GracePolicy, LicenceKey, LicencePayload, SignedFileSource, SignedLicence,
    Timestamp, TokenRedemptionSource, TrustBasis, Usability, VerifiedLicence, asserted_remotely,
    usability,
};

const DAY: i64 = 86_400;

fn at(day: i64) -> Timestamp {
    Timestamp::from_unix_seconds(day * DAY)
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[11_u8; 32])
}

fn licence_file(payload: &LicencePayload) -> Vec<u8> {
    let key = signing_key();
    let bytes = serde_json::to_vec(payload).unwrap();
    let signature = key.sign(&bytes);
    serde_json::to_vec(&SignedLicence::new(
        "file-key",
        bytes,
        signature.to_bytes().to_vec(),
    ))
    .unwrap()
}

fn endpoint() -> ActivationUrl {
    ActivationUrl::new("https://licences.example.com/v1").unwrap()
}

fn payload() -> LicencePayload {
    LicencePayload::new("example")
        .with_lease_until(at(30))
        .with_activation_id("slot-7")
}

// -- signed file ------------------------------------------------------------

#[test]
fn a_licence_file_settles_with_no_network_at_all() {
    let source = SignedFileSource::new(signing_key().verifying_key());

    let Activation::Settled(licence) = source
        .acquire(&Credential::LicenceFile(licence_file(&payload())))
        .unwrap()
    else {
        panic!("a licence file must settle locally");
    };

    assert!(licence.basis().is_offline_verifiable());
    assert_eq!(licence.payload().activation_id.as_deref(), Some("slot-7"));
}

#[test]
fn a_licence_file_signed_by_another_key_is_refused() {
    let source = SignedFileSource::new(SigningKey::from_bytes(&[3_u8; 32]).verifying_key());

    assert!(matches!(
        source.acquire(&Credential::LicenceFile(licence_file(&payload()))),
        Err(ActivationError::Verification(_))
    ));
}

#[test]
fn a_file_source_refuses_credentials_it_does_not_handle() {
    let source = SignedFileSource::new(signing_key().verifying_key());
    let key = LicenceKey::from_body("ABCDE12345").unwrap();

    assert_eq!(
        source.acquire(&Credential::Key(key)),
        Err(ActivationError::UnsupportedCredential)
    );
}

#[test]
fn a_file_source_holds_no_slot_to_release() {
    let source = SignedFileSource::new(signing_key().verifying_key());
    let licence = asserted_remotely(payload(), at(0));

    assert_eq!(source.release(&licence).unwrap(), Activation::Done);
}

// -- token redemption -------------------------------------------------------

#[test]
fn redeeming_a_key_describes_an_exchange_rather_than_performing_one() {
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key());
    let key = LicenceKey::from_body("ABCDE12345").unwrap();

    let Activation::Exchange(request) = source.acquire(&Credential::Key(key.clone())).unwrap()
    else {
        panic!("redemption needs the host to perform the exchange");
    };

    assert_eq!(request.url, endpoint());
    assert_eq!(
        request.headers,
        vec![("Content-Type".to_owned(), "application/json".to_owned())]
    );
    let body = String::from_utf8(request.body).unwrap();
    assert!(body.contains("redeem"), "{body}");
    assert!(body.contains(key.as_str()), "{body}");
}

#[test]
fn a_redemption_response_verifies_into_a_licence() {
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key());

    let licence = source.accept(&licence_file(&payload())).unwrap();

    assert!(licence.basis().is_offline_verifiable());
    assert_eq!(licence.payload().activation_id.as_deref(), Some("slot-7"));
}

#[test]
fn release_carries_the_activation_slot() {
    // Self-service release is the answer to the dominant support ticket, so
    // the interface has to express it rather than leaving each consumer to
    // invent one.
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key());
    let licence = asserted_remotely(payload(), at(0));

    let Activation::Exchange(request) = source.release(&licence).unwrap() else {
        panic!("release must reach the backend that holds the slot");
    };

    let body = String::from_utf8(request.body).unwrap();
    assert!(body.contains("release"), "{body}");
    assert!(body.contains("slot-7"), "{body}");
}

#[test]
fn renew_carries_the_activation_slot() {
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key());
    let licence = asserted_remotely(payload(), at(0));

    let Activation::Exchange(request) = source.renew(&licence).unwrap() else {
        panic!("renewal must reach the backend");
    };

    assert!(String::from_utf8(request.body).unwrap().contains("renew"));
}

// -- consumer adapters ------------------------------------------------------

/// A backend returning its own shape rather than a signed licence — the
/// hosted-service case Longhorn documents but does not ship.
struct HostedServiceSource;

impl ActivationSource for HostedServiceSource {
    fn acquire(&self, credential: &Credential) -> Result<Activation, ActivationError> {
        let Credential::AccountToken(token) = credential else {
            return Err(ActivationError::UnsupportedCredential);
        };
        Ok(Activation::Exchange(
            longhorn_licence::ActivationRequest::new(
                ActivationUrl::new("https://hosted.example.com/activate").unwrap(),
                Vec::new(),
            )
            .with_header("Authorization", format!("Bearer {token}")),
        ))
    }

    fn accept(&self, _response: &[u8]) -> Result<VerifiedLicence, ActivationError> {
        // No signature to check: the guarantee is the TLS session, so the
        // adapter must say so rather than claim one it does not have.
        Ok(asserted_remotely(payload(), at(1)))
    }
}

#[test]
fn a_consumer_adapter_declaring_a_remote_assertion_inherits_evaluation() {
    let licence = HostedServiceSource
        .accept(b"{}")
        .expect("a hosted adapter may assert without a signature");

    assert!(matches!(
        licence.basis(),
        TrustBasis::RemoteAssertion { .. }
    ));

    // And it inherits the weaker grace, with no extra wiring anywhere.
    assert_eq!(
        usability(
            &licence,
            at(40),
            ClockGuard::new(at(0)),
            GracePolicy::default()
        ),
        Usability::LeaseLapsed { at: at(37) }
    );
}

#[test]
fn a_source_that_never_requests_an_exchange_refuses_a_response() {
    let source = SignedFileSource::new(signing_key().verifying_key());

    assert_eq!(
        source.accept(b"{}"),
        Err(ActivationError::UnexpectedResponse)
    );
}

// -- transport --------------------------------------------------------------

#[test]
fn activation_endpoints_must_be_https() {
    assert_eq!(
        ActivationUrl::new("http://licences.example.com"),
        Err(ActivationUrlError::NotHttps)
    );
    assert_eq!(
        ActivationUrl::new("https://"),
        Err(ActivationUrlError::MissingHost)
    );
    assert!(ActivationUrl::new("https://licences.example.com").is_ok());
}
