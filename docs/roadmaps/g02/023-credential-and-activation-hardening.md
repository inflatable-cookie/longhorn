# g02.023 Credential And Activation Hardening

Status: ready
Owner: Tom
Updated: 2026-08-14
Governing refs: contract 019; contract 004; memo 023
Depends on: g02.010 (complete)

## Outcome

The credential path is as disciplined as the config-age path it sits beside:
activation payloads are built, not interpolated; secrets are zeroized and
redacted consistently; PKCE material can be generated instead of every
consumer copying a proof's timestamp stub; tampering is reported as tampering;
and the operational age identity has a Longhorn-owned persistence answer.

## Generation Runway

Memo 023's licence findings are individually small — an unescaped
interpolation here, a derived `Debug` there — but they share a shape: the
newest security surface has not yet had the consistency pass config-age got.
Contract 019 governs all of it; contract 004:403 already requires the
noninteractive identity authority Card 210 provides.

## Planning Gaps

- **`key_id` binding is a decision, not a fix.** A signed licence can claim
  any `key_id`; the signature verifies against the caller-supplied key either
  way. Whether projections should treat the field as rotation evidence is an
  adapter-domain call — Card 207 records it.
- **Age-identity storage shape.** `CredentialSlot` carries refresh tokens and
  licence keys only. Where the age identity lives (keyring slot vs config-age
  sidecar) is a boundary choice Card 210 makes against contract 004, not
  around it.

## Execution Plan

### Batch 1. The wire

- [ ] [Card 207](batch-cards/207-activation-payload-safety.md): redemption
  bodies built with `serde_json::json!`; metacharacter tests for token and
  `activation_id`; the `key_id` binding decision recorded.

### Batch 2. The memory and the message

- [ ] [Card 208](batch-cards/208-secret-hygiene-and-tamper-truth.md):
  `Zeroizing<String>` on the credential path; `Debug` redaction across licence
  types with tests; header-MAC tampering reports `Corrupt`, not `Locked`;
  `EncryptionFailed` keeps or drops the underlying `io::Error` by recorded
  decision.

### Batch 3. The operator-facing flow

- [ ] [Card 209](batch-cards/209-pkce-generation-and-loopback-robustness.md):
  CSPRNG `CodeVerifier::generate()`; the loopback listener survives a dead
  probe connection and enforces a total-connection deadline.
- [ ] [Card 210](batch-cards/210-age-identity-persistence.md): a Longhorn-owned
  persistence path for the operational age identity, satisfying contract
  004:403 without every consumer re-implementing the storage decision.

## Dependency Shape

```text
memo 023 (M-json, L-debug, L-locked, L-keyid, opp-zeroize, opp-pkce, opp-age-slot)
             + presentation lane (loopback probe abort, trickle deadline)
 └─ 023 credential and activation hardening
     ├─ 207 activation payload   (independent)
     ├─ 208 secret hygiene       (independent)
     ├─ 209 pkce + loopback      (independent)
     └─ 210 age-identity slot    (boundary choice; lands after 208 settles slots)
```

## Goals

- [ ] no credential crosses a serialization boundary by string interpolation
- [ ] a `{:?}` anywhere in the tree cannot print a bearer token
- [ ] the pattern a consumer copies for PKCE is a safe one
- [ ] automatic encrypted backup has its noninteractive identity authority

## Acceptance Criteria

- [ ] redemption and renewal bodies escape correctly for adversarial tokens,
  with tests
- [ ] redaction tests assert `Debug` output of credential-carrying types
  contains no secret, matching the config-age precedent
- [ ] tamper-after-header-MAC classifies as `Corrupt` with a regression test
- [ ] contract 004:403's requirement names the mechanism that satisfies it

## Explicit Non-goals

- Re-plumbing the `CredentialStore` trait boundary. The OS keychain stays the
  boundary; zeroization hardens what crosses it, it does not move it.
- Clock-guard changes. Fail-open within lease is contract 019 policy, audited
  and confirmed deliberate.

## Next Task

Card 207. It is the only finding in this milestone with an injection class,
however narrow.

## Planning Checkpoint

After Batch 2. If `zeroize`'s addition to the dependency set raises floor or
advisory questions, settle them there rather than per-crate.
