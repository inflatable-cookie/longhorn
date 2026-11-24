//! Activation adapter acceptance evidence.

use ed25519_dalek::{Signer, SigningKey};
use longhorn_licence::{
    AccountFlow, Activation, ActivationError, ActivationSource, ActivationUrl, ActivationUrlError,
    ClockGuard, CodeVerifier, Credential, GracePolicy, LicenceCredentialProjection, LicenceKey,
    LicencePayload, SignedFileSource, SignedLicence, Timestamp, TokenRedemptionSource, TrustBasis,
    Usability, VerificationError, VerifiedLicence, usability,
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
    let source = SignedFileSource::new(signing_key().verifying_key(), "example");

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
    let source = SignedFileSource::new(
        SigningKey::from_bytes(&[3_u8; 32]).verifying_key(),
        "example",
    );

    assert!(matches!(
        source.acquire(&Credential::LicenceFile(licence_file(&payload()))),
        Err(ActivationError::Verification(_))
    ));
}

#[test]
fn a_licence_file_for_another_product_is_refused() {
    let source = SignedFileSource::new(signing_key().verifying_key(), "other-product");

    assert_eq!(
        source.acquire(&Credential::LicenceFile(licence_file(&payload()))),
        Err(ActivationError::Verification(
            VerificationError::ProductMismatch {
                expected: "other-product".to_owned(),
                actual: "example".to_owned(),
            }
        ))
    );
}

#[test]
fn a_claimed_key_id_is_recorded_as_a_claim_not_evidence() {
    // The envelope key id is not covered by the signature: a licence naming
    // a retired key still verifies against the right key, and the trust
    // basis keeps the claim visibly a claim. Pinned so the behavior is
    // chosen, not accidental.
    let source = SignedFileSource::new(signing_key().verifying_key(), "example");
    let key = signing_key();
    let bytes = serde_json::to_vec(&payload()).unwrap();
    let signature = key.sign(&bytes);
    let named_otherwise = serde_json::to_vec(&SignedLicence::new(
        "retired-key",
        bytes,
        signature.to_bytes().to_vec(),
    ))
    .unwrap();

    let Activation::Settled(licence) = source
        .acquire(&Credential::LicenceFile(named_otherwise))
        .unwrap()
    else {
        panic!("a licence naming any key id verifies against the real key");
    };

    assert!(matches!(
        licence.basis(),
        TrustBasis::OfflineSignature { key_id } if key_id == "retired-key"
    ));
}

#[test]
fn a_file_source_refuses_credentials_it_does_not_handle() {
    let source = SignedFileSource::new(signing_key().verifying_key(), "example");
    let key = LicenceKey::from_body("ABCDE12345FGHJK6789").unwrap();

    assert_eq!(
        source.acquire(&Credential::Key(key)),
        Err(ActivationError::UnsupportedCredential)
    );
}

#[test]
fn a_file_source_holds_no_slot_to_release() {
    let source = SignedFileSource::new(signing_key().verifying_key(), "example");
    let licence = VerifiedLicence::from_remote_assertion(payload(), at(0));

    assert_eq!(source.release(&licence).unwrap(), Activation::Done);
}

// -- token redemption -------------------------------------------------------

#[test]
fn redeeming_a_key_describes_an_exchange_rather_than_performing_one() {
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");
    let key = LicenceKey::from_body("ABCDE12345FGHJK6789").unwrap();

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
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");

    let licence = source.accept(&licence_file(&payload())).unwrap();

    assert!(licence.basis().is_offline_verifiable());
    assert_eq!(licence.payload().activation_id.as_deref(), Some("slot-7"));
}

#[test]
fn a_redemption_response_for_another_product_is_refused() {
    let source =
        TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "other-product");

    assert_eq!(
        source.accept(&licence_file(&payload())),
        Err(ActivationError::Verification(
            VerificationError::ProductMismatch {
                expected: "other-product".to_owned(),
                actual: "example".to_owned(),
            }
        ))
    );
}

#[test]
fn release_carries_the_activation_slot() {
    // Self-service release is the answer to the dominant support ticket, so
    // the interface has to express it rather than leaving each consumer to
    // invent one.
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");
    let licence = VerifiedLicence::from_remote_assertion(payload(), at(0));

    let Activation::Exchange(request) = source.release(&licence).unwrap() else {
        panic!("release must reach the backend that holds the slot");
    };

    let body = String::from_utf8(request.body).unwrap();
    assert!(body.contains("release"), "{body}");
    assert!(body.contains("slot-7"), "{body}");
}

#[test]
fn renew_carries_the_activation_slot() {
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");
    let licence = VerifiedLicence::from_remote_assertion(payload(), at(0));

    let Activation::Exchange(request) = source.renew(&licence).unwrap() else {
        panic!("renewal must reach the backend");
    };

    assert!(String::from_utf8(request.body).unwrap().contains("renew"));
}

#[test]
fn renew_and_release_fail_when_activation_id_is_missing() {
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");
    let without_id = VerifiedLicence::from_remote_assertion(
        LicencePayload::new("example").with_lease_until(at(30)),
        at(0),
    );
    let empty_id = VerifiedLicence::from_remote_assertion(
        LicencePayload::new("example")
            .with_lease_until(at(30))
            .with_activation_id(""),
        at(0),
    );

    for licence in [&without_id, &empty_id] {
        assert_eq!(
            source.renew(licence),
            Err(ActivationError::MissingActivationId)
        );
        assert_eq!(
            source.release(licence),
            Err(ActivationError::MissingActivationId)
        );
    }
}

#[test]
fn json_metacharacters_in_a_token_cannot_reshape_the_request() {
    // The body is built with a serializer, not interpolation: a token
    // containing a quote must survive as data, not become structure. An
    // injection attempt is the same test, because it is the same bytes.
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");
    let hostile = r#"x","action":"release"#;

    let Activation::Exchange(request) = source
        .acquire(&Credential::AccountToken(secrecy::SecretString::from(
            hostile.to_owned(),
        )))
        .unwrap()
    else {
        panic!("activation composes an exchange");
    };

    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["action"], "activate");
    assert_eq!(body["value"], hostile);
}

#[test]
fn a_backend_issued_activation_id_with_metacharacters_stays_data() {
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");
    let mut shaped = payload();
    shaped.activation_id = Some(r#"slot-7\"""#.to_owned());
    let licence = VerifiedLicence::from_remote_assertion(shaped, at(0));

    for exchange in [source.renew(&licence), source.release(&licence)] {
        let Activation::Exchange(request) = exchange.unwrap() else {
            panic!("renew and release compose exchanges");
        };
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["value"], r#"slot-7\"""#);
    }
}

#[test]
fn debug_on_the_credential_path_prints_no_secret() {
    // One `{:?}` in a handler must not be a way to carry a bearer token out.
    // The wire projection keeps `String` (it crosses inward) and redacts;
    // the domain types hold `SecretString` and redact by construction.
    let token = "bearer-token-0123456789";
    let credential = Credential::AccountToken(secrecy::SecretString::from(token.to_owned()));
    assert!(!format!("{credential:?}").contains(token));

    let verifier_secret = "proof-verifier-0123456789-0123456789-0123456789";
    let state_secret = "proof-state-sixteen-bytes";
    let verifier = CodeVerifier::new(verifier_secret).unwrap();
    assert!(!format!("{verifier:?}").contains(verifier_secret));
    let flow = AccountFlow::begin(verifier, state_secret, 9876).unwrap();
    let printed = format!("{flow:?}");
    assert!(!printed.contains(state_secret));
    assert!(!printed.contains(verifier_secret));

    for projection in [
        LicenceCredentialProjection::Key {
            key: "ABCDE12345FGHJK6789X".to_owned(),
        },
        LicenceCredentialProjection::AccountToken {
            token: token.to_owned(),
        },
        LicenceCredentialProjection::LicenceFile {
            contents_base64: "AAEC".to_owned(),
        },
    ] {
        let printed = format!("{projection:?}");
        assert!(!printed.contains("ABCDE12345"));
        assert!(!printed.contains(token));
        assert!(!printed.contains("AAEC"));
    }
}

#[test]
fn debug_on_an_activation_exchange_prints_no_credential() {
    // Redemption puts the key or account token in the request body, and a
    // consumer adapter may put a bearer token in a header value. `{:?}` on
    // either the request or the enclosing exchange must not carry any of
    // that out. Derived `Debug` would print `body: [123, 34, ...]`, from
    // which the credential decodes byte for byte.
    let source = TokenRedemptionSource::new(endpoint(), signing_key().verifying_key(), "example");
    let key = LicenceKey::from_body("ABCDE12345FGHJK6789").unwrap();
    let token = "bearer-token-0123456789";

    let key_exchange = source.acquire(&Credential::Key(key.clone())).unwrap();
    let token_exchange = source
        .acquire(&Credential::AccountToken(secrecy::SecretString::from(
            token.to_owned(),
        )))
        .unwrap();
    let bearer_exchange = Activation::Exchange(
        longhorn_licence::ActivationRequest::new(
            ActivationUrl::new("https://hosted.example.com/activate").unwrap(),
            Vec::new(),
        )
        .with_header("Authorization", format!("Bearer {token}")),
    );

    for exchange in [&key_exchange, &token_exchange, &bearer_exchange] {
        let Activation::Exchange(request) = exchange else {
            panic!("these sources compose exchanges");
        };
        for printed in [format!("{request:?}"), format!("{exchange:?}")] {
            assert!(!printed.contains(key.as_str()), "{printed}");
            assert!(!printed.contains(token), "{printed}");
            assert!(!printed.contains("application/json"), "{printed}");
            assert!(
                !request_body_is_recoverable(&printed, &request.body),
                "{printed}"
            );
            // Diagnostics survive the scrub: URL, body length, header names.
            assert!(
                printed.contains("licences.example.com") || printed.contains("hosted.example.com"),
                "{printed}"
            );
            assert!(printed.contains("body_len"), "{printed}");
        }
        let printed = format!("{request:?}");
        assert!(!printed.contains("body: ["), "{printed}");
        for (name, _) in &request.headers {
            assert!(printed.contains(name), "{printed}");
        }
    }
}

/// Replays the derived-`Debug` leak: a `body: [n, n, ...]` byte list that
/// decodes back to the request body.
fn request_body_is_recoverable(printed: &str, body: &[u8]) -> bool {
    let Some(start) = printed.find("body: [") else {
        return false;
    };
    let rest = &printed[start + "body: [".len()..];
    let Some(end) = rest.find(']') else {
        return false;
    };
    let decoded: Option<Vec<u8>> = rest[..end]
        .split(',')
        .map(|part| part.trim().parse::<u8>().ok())
        .collect();
    decoded.as_deref() == Some(body)
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
            .with_header(
                "Authorization",
                format!("Bearer {}", secrecy::ExposeSecret::expose_secret(token)),
            ),
        ))
    }

    fn accept(&self, _response: &[u8]) -> Result<VerifiedLicence, ActivationError> {
        // No signature to check: the guarantee is the TLS session, so the
        // adapter must say so rather than claim one it does not have.
        Ok(VerifiedLicence::from_remote_assertion(payload(), at(1)))
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
    let source = SignedFileSource::new(signing_key().verifying_key(), "example");

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
