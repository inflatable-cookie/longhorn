//! Headless update and licence flow proof (Card 159).
//!
//! Exercises every pure decision, gate authorization, verification,
//! activation, usability, and credential claim that can be proved without a
//! packaged application. Longhorn is the update controller for both hosts
//! (operator decision, 2026-08-12); the packaged claims this harness cannot
//! make live in `packaged-update-proof` and `tauri-update-proof`, and the
//! claims block in `main.rs` says where each one's evidence is.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use longhorn_browser::LoopbackRedirect;
use longhorn_core::{CredentialSlot, CredentialStore, MemoryCredentialStore};
use longhorn_licence::{
    AccountFlow, Activation, ActivationSource, ActivationUrl, ClockGuard, CodeVerifier, Credential,
    GracePolicy, LicencePayload, SignedFileSource, SignedLicence, Span, Timestamp,
    TokenRedemptionSource, TrustBasis, Usability, asserted_remotely, usability, verify,
};
use longhorn_update::{
    Artifact, BuildIdentity, Channel, ChannelManifest, CheckKind, DeferralCause, EndpointUrl,
    InstallId, InstallProvenance, QuiescenceKind, Rollout, RolloutFraction, StaticJsonSource,
    TargetTriple, UpdateAvailability, UpdateSource, evaluate,
};
use longhorn_update::{CountingProbe, InstallAuthorization, UpdateGate, transfer_session_probe};
use semver::Version;
use serde_json::{Value, json};

/// Produces the update-flow evidence record.
///
/// Every claim is asserted inline: a regression fails the harness loudly
/// instead of being recorded as a pass.
#[must_use]
pub fn update_evidence() -> Value {
    let version_1_0_0 = Version::new(1, 0, 0);
    let version_1_1_0 = Version::new(1, 1, 0);
    let build = BuildIdentity::new(Channel::Production, version_1_0_0.clone());
    let install = InstallId::new("install-0001").unwrap();

    let up_to_date = ChannelManifest::new(Channel::Production, version_1_0_0.clone());
    assert!(matches!(
        evaluate(
            &build,
            &up_to_date,
            &install,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged
        ),
        UpdateAvailability::UpToDate
    ));

    let ahead = ChannelManifest::new(Channel::Production, Version::new(0, 9, 0));
    assert!(matches!(
        evaluate(
            &build,
            &ahead,
            &install,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged
        ),
        UpdateAvailability::AheadOfChannel { .. }
    ));

    let below_minimum = ChannelManifest::new(Channel::Production, version_1_1_0.clone())
        .with_minimum_version(Version::new(1, 2, 0));
    assert!(matches!(
        evaluate(
            &build,
            &below_minimum,
            &install,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged
        ),
        UpdateAvailability::Offer(_)
    ));

    let withheld = ChannelManifest::new(Channel::Production, version_1_1_0.clone()).with_rollout(
        Rollout::new(RolloutFraction::new(0.0).unwrap(), "proof-seed"),
    );
    assert!(matches!(
        evaluate(
            &build,
            &withheld,
            &install,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged
        ),
        UpdateAvailability::WithheldByRollout { .. }
    ));
    assert!(matches!(
        evaluate(
            &build,
            &withheld,
            &install,
            CheckKind::UserInitiated,
            InstallProvenance::SelfManaged
        ),
        UpdateAvailability::Offer(_)
    ));

    let full_rollout = ChannelManifest::new(Channel::Production, version_1_1_0.clone())
        .with_rollout(Rollout::new(
            RolloutFraction::new(1.0).unwrap(),
            "proof-seed",
        ))
        .with_artifact(
            TargetTriple::new("aarch64-apple-darwin").unwrap(),
            Artifact::new("https://updates.example/1.1.0.tar.gz", "signature"),
        );
    assert!(matches!(
        evaluate(
            &build,
            &full_rollout,
            &install,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged
        ),
        UpdateAvailability::Offer(_)
    ));

    let loopback = EndpointUrl::new("http://127.0.0.1:38473/update").unwrap();
    let source = StaticJsonSource::new(loopback.as_str());
    let request = source.manifest_request(Channel::Production).unwrap();
    assert_eq!(
        request.url.as_str(),
        "http://127.0.0.1:38473/update/production.json"
    );

    let idle = transfer_session_probe(|| 0);
    let quiescent: Vec<&dyn longhorn_update::QuiescenceProbe> = vec![&idle];
    let gate = UpdateGate::new(quiescent);
    assert_eq!(
        gate.authorize(&version_1_1_0),
        InstallAuthorization::Approved
    );

    let busy = transfer_session_probe(|| 1);
    let busy_probes: Vec<&dyn longhorn_update::QuiescenceProbe> = vec![&busy];
    let gate = UpdateGate::new(busy_probes);
    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version_1_1_0) else {
        panic!("an open transfer session must refuse the install");
    };
    assert!(matches!(deferral.cause, DeferralCause::WorkInFlight { .. }));
    assert_eq!(deferral.version, version_1_1_0);

    let flushes = CountingProbe::new(QuiescenceKind::PendingFlush, || 2);
    let sessions = transfer_session_probe(|| 1);
    let operations = longhorn_update::operation_probe(|| 3);
    let probes: Vec<&dyn longhorn_update::QuiescenceProbe> = vec![&flushes, &sessions, &operations];
    let receipt = UpdateGate::new(probes).quiescence();
    assert_eq!(
        receipt.detail(),
        "2 pending flushes, 1 open transfer session, 3 running operations"
    );

    json!({
        "decision": {
            "upToDate": "proved",
            "aheadOfChannel": "proved",
            "belowMinimumOffers": "proved",
            "rolloutWithholdsAutomaticAndYieldsToUserInitiated": "proved",
            "fullRolloutOffers": "proved",
        },
        "source": {
            "staticJsonLoopbackEndpointAccepted": "proved",
        },
        "gate": {
            "quiescentHostAuthorizesTheInstall": "proved",
            "openTransferSessionRefusesWithWorkInFlight": "proved",
            "refusalCarriesVersionAndReason": "proved",
            "receiptReportsEveryOutstandingItem": "proved",
        },
    })
}

/// Produces the licence-flow evidence record.
#[must_use]
pub fn licence_evidence() -> Value {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let verifying_key: VerifyingKey = signing_key.verifying_key();
    let now = Timestamp::from_unix_seconds(1_750_000_000);

    let payload = LicencePayload {
        use_until: None,
        update_until: None,
        lease_until: None,
        ..LicencePayload::new("proof-product")
    };
    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let signature: Signature = signing_key.sign(&payload_bytes);

    let signed = SignedLicence::new(
        "proof-key",
        payload_bytes.clone(),
        signature.to_bytes().to_vec(),
    );
    let verified = verify(&signed, &verifying_key).expect("a genuine signature must verify");
    assert_eq!(verified.payload().product, "proof-product");

    let mut tampered = payload_bytes.clone();
    tampered[0] ^= 0x01;
    let forged = SignedLicence::new("proof-key", tampered, signature.to_bytes().to_vec());
    assert!(matches!(
        verify(&forged, &verifying_key),
        Err(longhorn_licence::VerificationError::SignatureRejected)
    ));

    let wrong_key = SigningKey::from_bytes(&[9; 32]).verifying_key();
    assert!(verify(&signed, &wrong_key).is_err());

    let file_source = SignedFileSource::new(verifying_key);
    let Activation::Settled(file_licence) = file_source
        .acquire(&Credential::LicenceFile(
            serde_json::to_vec(&signed).unwrap(),
        ))
        .unwrap()
    else {
        panic!("a licence file must settle locally");
    };
    assert!(matches!(
        file_licence.basis(),
        TrustBasis::OfflineSignature { .. }
    ));

    let token_source = TokenRedemptionSource::new(
        ActivationUrl::new("https://stub.example/redeem").unwrap(),
        verifying_key,
    );
    let redemption_key = longhorn_licence::LicenceKey::from_body("ABCDE12345FGHJK6789")
        .expect("a well-formed redemption key parses");
    let Activation::Exchange(request) = token_source
        .acquire(&Credential::Key(redemption_key))
        .unwrap()
    else {
        panic!("a token redemption must request an exchange");
    };
    assert_eq!(request.url.as_str(), "https://stub.example/redeem");
    let redeemed = token_source
        .accept(&serde_json::to_vec(&signed).unwrap())
        .expect("a signed redemption response must settle");
    assert!(matches!(
        redeemed.basis(),
        TrustBasis::OfflineSignature { .. }
    ));

    let active = asserted_remotely(
        LicencePayload {
            lease_until: Some(now),
            ..payload.clone()
        },
        now,
    );
    let grace = GracePolicy::new(Span::from_days(14), Span::from_days(14));
    assert!(matches!(
        usability(&active, now, ClockGuard::new(now), grace),
        Usability::Active
    ));

    let day = 86_400;
    let expired = asserted_remotely(
        LicencePayload {
            use_until: Some(Timestamp::from_unix_seconds(now.as_unix_seconds() - day)),
            ..payload.clone()
        },
        now,
    );
    assert!(matches!(
        usability(&expired, now, ClockGuard::new(now), grace),
        Usability::UseWindowExpired { .. }
    ));

    let lapsed_lease = asserted_remotely(
        LicencePayload {
            lease_until: Some(Timestamp::from_unix_seconds(
                now.as_unix_seconds() - 30 * day,
            )),
            ..payload.clone()
        },
        now,
    );
    assert!(matches!(
        usability(&lapsed_lease, now, ClockGuard::new(now), grace),
        Usability::LeaseLapsed { .. }
    ));

    let clocked_back = asserted_remotely(payload.clone(), now);
    let guard = ClockGuard::new(now.saturating_add(Span::from_days(10)));
    assert!(matches!(
        usability(&clocked_back, now, guard, grace),
        Usability::ClockRefused
    ));

    // The platform half of the credential claims, pending since 2026-08-08
    // and unblocked by the CredentialStore decision of 2026-08-14: a real
    // keychain entry, stored by one process and read by a later one.
    //
    // Restart persistence cannot be proved inside a single run -- the claim
    // is precisely that the value outlives the process -- so the harness
    // leaves a marker behind and each run reports what it found. The first
    // run stores and says so; every later run finds the previous run's marker
    // and the claim is met.
    let restart_persistence = platform_persistence_claim();

    let store = MemoryCredentialStore::new();
    store.store(CredentialSlot::RefreshToken, "token").unwrap();
    store.store(CredentialSlot::LicenceKey, "key").unwrap();
    assert_eq!(
        store.retrieve(CredentialSlot::RefreshToken).unwrap(),
        Some("token".to_owned())
    );
    assert_eq!(
        store.retrieve(CredentialSlot::LicenceKey).unwrap(),
        Some("key".to_owned())
    );
    store.remove(CredentialSlot::RefreshToken).unwrap();
    assert_eq!(store.retrieve(CredentialSlot::RefreshToken).unwrap(), None);
    store
        .remove(CredentialSlot::RefreshToken)
        .expect("removing an empty slot succeeds");

    json!({
        "verification": {
            "genuineSignatureVerifies": "proved",
            "tamperedPayloadRejected": "proved",
            "wrongKeyRejected": "proved",
        },
        "activation": {
            "signedFileSettlesLocally": "proved",
            "tokenRedemptionRequestsExchangeAndSettles": "proved",
        },
        "usability": {
            "activeWithinLease": "proved",
            "expiredUseWindowBlocks": "proved",
            "lapsedLeaseBlocks": "proved",
            "clockRollbackRefuses": "proved",
        },
        "account": {
            "loopbackRedirectRoundTrips": loopback_account_claim(),
        },
        "credentials": {
            "slotsRoundTripAndRemovalIsIdempotent": "proved",
            "restartPersistence": restart_persistence,
        },
    })
}

/// One real-keychain round trip, plus the cross-run marker.
///
/// Uses a proof-specific service name so it can never touch a consumer's
/// entries. The marker is deliberately not a secret -- it is the run stamp of
/// whichever run wrote it, so a reader of the evidence can see how far apart
/// the two processes were.
fn platform_persistence_claim() -> String {
    use longhorn_credential_keyring::KeyringCredentialStore;

    let store = KeyringCredentialStore::new("audio.example.longhorn-update-licence-proof");
    let slot = CredentialSlot::RefreshToken;

    // The in-process contract first: store, read back, replace. A backend
    // that cannot do this much has no persistence to claim.
    if let Err(error) = store.store(slot, "proof-round-trip") {
        return format!("unavailable: {error}");
    }
    match store.retrieve(slot) {
        Ok(Some(value)) if value == "proof-round-trip" => {}
        Ok(other) => return format!("round trip returned {other:?}"),
        Err(error) => return format!("unavailable: {error}"),
    }

    // The cross-run marker, in the other slot so the round trip above cannot
    // be mistaken for it.
    let marker = CredentialSlot::LicenceKey;
    let previous = match store.retrieve(marker) {
        Ok(found) => found,
        Err(error) => return format!("unavailable: {error}"),
    };
    let stamp = format!("run:{}", std::process::id());
    if let Err(error) = store.store(marker, &stamp) {
        return format!("unavailable: {error}");
    }
    // The round-trip slot is cleaned; the marker stays for the next run.
    drop(store.remove(slot));

    match previous {
        Some(earlier) => {
            format!("proved - this run read `{earlier}` written by an earlier process")
        }
        None => "stored by this run - proved on the next".to_owned(),
    }
}

/// The RFC 8252 chain minus the human: flow, listener, redirect, acceptance.
///
/// `AccountFlow` builds the PKCE challenge and the redirect URI against the
/// listener's real port; a thread plays the browser's part by delivering the
/// redirect over an actual socket; `accept_callback` checks state in constant
/// time and yields the authorization. What remains for a packaged run is only
/// the system browser itself -- `longhorn-browser` owns the launch, and an
/// operator owns the click.
fn loopback_account_claim() -> String {
    let listener = match LoopbackRedirect::bind() {
        Ok(listener) => listener,
        Err(error) => return format!("bind failed: {error}"),
    };
    let port = listener.port();
    // A fixed verifier is fine here: the claim is composition, not entropy,
    // and `CodeVerifier` enforces RFC 7636's own length floor.
    let verifier = match CodeVerifier::new("proof-verifier-0123456789-0123456789-0123456789") {
        Ok(verifier) => verifier,
        Err(error) => return format!("verifier refused: {error}"),
    };
    let flow = match AccountFlow::begin(verifier, "proof-state-sixteen-bytes", port) {
        Ok(flow) => flow,
        Err(error) => return format!("flow refused: {error}"),
    };

    let redirect = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", port)).expect("the listener is bound");
        stream
            .write_all(
                b"GET /callback?state=proof-state-sixteen-bytes&code=proof-code HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            )
            .expect("redirect delivered");
        let mut page = String::new();
        drop(stream.read_to_string(&mut page));
        page
    });

    let callback = match listener.receive(std::time::Duration::from_secs(5)) {
        Ok(callback) => callback,
        Err(error) => return format!("no callback: {error}"),
    };
    let page = redirect.join().expect("redirect thread");
    if !page.contains("200 OK") {
        return "the browser page did not come back".to_owned();
    }
    match flow.accept_callback(&callback) {
        Ok(_) => "proved - flow, listener, socket redirect and state acceptance compose".to_owned(),
        Err(error) => format!("callback refused: {error}"),
    }
}
