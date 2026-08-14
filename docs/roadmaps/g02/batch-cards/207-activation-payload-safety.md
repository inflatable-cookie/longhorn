# 207 Activation Payload Safety

Status: ready
Owner: Tom
Roadmap: g02.023 batch 1
Governing refs: contract 019; memo 023 (M-json, L-keyid)
Depends on: none
Auto-start next card: no

## Objective

Activation payloads are built with a serializer, not string interpolation,
and the `key_id`-as-rotation-evidence question is decided and recorded.

## Why this exists

`TokenRedemptionSource::exchange` builds its request body with
`format!(r#"{{"action":"{action}","value":"{payload}"}}"#)`
(`crates/longhorn-licence/src/activation.rs:277-283`). For
`Credential::AccountToken` the payload is an arbitrary client-boundary string
(`protocol.rs:319-324`): a token containing `"` or `\` produces malformed JSON
at best and duplicate-key field injection into the redemption endpoint at
worst. The `Credential::Key` path is safe only incidentally (Crockford
normalization restricts the alphabet). `renew`/`release` embed the signed
licence's `activation_id` the same way. No test covers JSON metacharacters.

Separately, `verify()` records the attacker-controlled `key_id` verbatim into
`TrustBasis::OfflineSignature` (`verify.rs:67-70`). The signature checks
against the caller-supplied key — correct — but the projection treats
`key_id` as rotation evidence, and a licence can name a retired key.

## Scope

- `crates/longhorn-licence/src/activation.rs` — body construction
- `crates/longhorn-licence/src/verify.rs` — the `key_id` decision
- contract 019, if the decision changes what projections may claim

## Steps

1. Replace the `format!` bodies with `serde_json::json!` construction for
   exchange, renew, and release.
2. Tests: tokens and activation ids containing `"`, `\`, newlines, and a
   duplicate-key injection attempt (`x","action":"release`).
3. Decide `key_id`: keep as opaque metadata, bind it to the verifying key, or
   drop it from rotation reasoning. Record the decision where projections
   consume it; add the mismatched-`key_id` negative test.
4. Contract 019 amendment only if the decision changes a stated rule.

## Do Not

- Hand-escape the strings. The fix is a serializer, not a better `replace`.
- Treat the Key path's incidental safety as coverage — test the AccountToken
  path directly.

## Acceptance Criteria

- [ ] all three bodies are serializer-built
- [ ] metacharacter and injection-attempt tests pass
- [ ] the `key_id` decision is recorded with its negative test

## Evidence Required

- the diff on body construction
- the new tests
- `effigy qa` green

## Stop Conditions

None anticipated. If the `key_id` decision turns out to need consumer input
(someone already keys rotation on it), surface it rather than guessing.
