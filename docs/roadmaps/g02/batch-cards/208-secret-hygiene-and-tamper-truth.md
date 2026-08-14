# 208 Secret Hygiene And Tamper Truth

Status: complete
Completed: 2026-08-14
Owner: Tom
Roadmap: g02.023 batch 2
Governing refs: contract 019; contract 004; memo 023 (L-debug, L-locked,
L5-config-age)
Depends on: none
Auto-start next card: no

## Objective

The licence crate's secret handling matches the config-age precedent: secrets
zeroized, `Debug` redacted and tested, and tampering reported as tampering.

## Why this exists

Three consistency findings against the sibling crate's deliberate discipline:

- `Credential` (`crates/longhorn-licence/src/activation.rs:125`),
  `AccountFlow` (`account.rs:69`), `CodeVerifier` (`account.rs:22`), and
  `LicenceCredentialProjection` (`protocol.rs:306`) derive `Debug` over bearer
  tokens, the PKCE verifier, and base64 licence files. Nothing logs them
  today — latent — but one future `{:?}` in a handler exposes a token, in
  direct contrast to `AgeIdentity`/`AgePassphrase` printing `<redacted>`
  (`longhorn-config-age/src/authority.rs:76-80,113-117`).
- Secrets cross the `CredentialStore` trait as plain `String`; `zeroize` is
  almost certainly already in the tree transitively via age/ed25519-dalek.
- `age::DecryptError::DecryptionFailed` maps to `Locked`
  (`crates/longhorn-config-age/src/inspection.rs:184-194`), but that error
  after a matched stanza means header-MAC / payload authentication failure —
  tampering — which `types.rs:162-163` documents as `Corrupt`. The existing
  tamper test only hits `Corrupt` incidentally, via the STREAM reader.
  Related: `AgeEncryptionError::EncryptionFailed` discards the underlying
  `io::Error` (`envelope.rs:122`), unremarked.

## Scope

- `crates/longhorn-licence` — redaction, zeroization
- `crates/longhorn-config-age` — tamper classification, error fidelity
- `crates/longhorn-credential-keyring` — trait signature if zeroization
  crosses it

## Steps

1. `Zeroizing<String>` for `CredentialStore` secrets and
   `Credential::AccountToken`; confirm `zeroize`'s presence in the tree or add
   it at workspace level.
2. Hand-written redacting `Debug` for the four licence types, matching the
   config-age pattern.
3. Redaction tests: `format!("{:?}", …)` output contains no secret material
   for each type.
4. Fix the tamper classification: header-authentication failure with a valid
   identity reports `Corrupt`; regression test for header-MAC tamper (the
   existing test flips a payload byte — add the header case).
5. `EncryptionFailed`: keep the underlying error or drop it — decide, comment,
   test the message either way.

## Do Not

- Zeroize across the OS keychain boundary itself — the keychain is the
  boundary; this card hardens what crosses it.
- Change `MemoryCredentialStore`'s no-persistence contract.

## Result

The licence crate now holds its secrets the way config-age holds its age
material: `Credential::AccountToken`, `CodeVerifier`, and `AccountFlow.state`
are `SecretString` (secrecy 0.10.3, already in the tree via age — added as a
direct workspace dependency), zeroized on drop and redacted in `Debug` by
construction. Equality on those types is constant-time, matching the
callback-state comparison's precedent. `LicenceCredentialProjection` keeps
its wire `String` (it crosses inward) and redacts `Debug` by hand. A test
asserts no secret survives a `{:?}` on any of the four types.

**The tamper finding was investigated and disproven as fixable, and the
record now says why.** The audit read `DecryptError::DecryptionFailed` as
"stanza matched, then tampering detected". In age's implementation a
tampered header stanza yields `NoMatchingKeys` — indistinguishable from a
wrong key (verified with a live probe: flipping one stanza character with
the correct identity present produces `Locked`). A tampered *payload* does
surface as `Corrupt`, via the stream read path, and the existing test covers
it. So the classifier stands, and `classify_decrypt_error` now documents
what `Locked` honestly means: "this key did not open this file", not "the
key is wrong". The classifier change was written, probed, and reverted; the
doc is the fix.

`EncryptionFailed` keeps the underlying io error redacted, now with the
reason written at the site (boundary detail can name recipient material;
the remedy is the same either way) rather than as a silent drop.

**Residual, recorded:** the `CredentialStore` trait keeps plain `String` at
the OS-keychain boundary — the keychain is the boundary, and changing the
trait's types is a consumer-coordinated change, not this card's.

## Acceptance Criteria

- [x] no `Debug` impl in the tree can print a credential, with tests
- [x] secrets on the credential path are zeroized on drop
- [x] header tampering investigated: indistinguishable at age's error
  surface; the honest meaning of `Locked` is documented (see Result)
- [x] the `EncryptionFailed` fidelity decision is recorded

## Evidence Required

- the redaction tests' assertions
- dependency diff (`zeroize`)
- `effigy qa` green

## Stop Conditions

Stop if `Zeroizing` on the trait object surface forces a breaking change to a
consumer-implemented `CredentialStore` — that coordinates like any consumer
break.
