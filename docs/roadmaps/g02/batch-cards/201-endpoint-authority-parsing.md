# 201 Endpoint Authority Parsing

Status: ready
Owner: Tom
Roadmap: g02.021 batch 1
Governing refs: contract 018; memo 023 (H2); PAPERCUTS (duplicated URL
validation)
Depends on: none
Auto-start next card: no

## Objective

No authority string classifies a remote host as loopback, and a mislabeled
manifest cannot silently restage a rollout.

## Why this exists

`is_loopback_host` (`crates/longhorn-update/src/source.rs:51-63`) splits the
authority on `:` / `]` without stripping userinfo. `http://127.0.0.1:80@evil.
example/x` parses as host `127.0.0.1` and is accepted; the fetch goes to a
remote host over plaintext, defeating the gate `source.rs:9-14` documents as
load-bearing. Tests (`:340-363`) cover prefix tricks, not userinfo. Separately,
`evaluate` never checks `manifest.channel == build.channel`
(`decision.rs:123-179`), so a mislabeled manifest silently drives rollout
staging.

## Scope

- `crates/longhorn-update/src/source.rs` — authority parsing
- `crates/longhorn-update/src/decision.rs` — the channel check
- review `crates/longhorn-licence/src/activation.rs`'s `ActivationUrl` for the
  same parsing class (the papercut says the thirty lines are the same)

## Steps

1. Parse the authority properly: strip userinfo before the loopback match, or
   adopt a URL parser and record why. Keep the loopback-HTTP exception exactly
   as contract 018 states it — this card closes the bypass, not the exception.
2. Tests: `127.0.0.1:80@evil.example`, `[::1]@evil.example`,
   `user@localhost`, empty userinfo, userinfo with encoded characters.
3. Add the channel check at `evaluate`: manifest channel must equal the build
   channel; mismatch is a classified refusal, not a silent restage.
4. Diff `ActivationUrl`'s parsing against the fixed `EndpointUrl`. If they can
   share a primitive without coupling the crates, note it; the papercut's
   disposition (wait for a third caller) stands unless the fix changes that.

## Do Not

- Remove the loopback exception. The local shim is deliberate.
- Reach for a full URL crate without checking what it does to the workspace
  dependency set — a hand fix on one function may be the right size.

## Acceptance Criteria

- [ ] every userinfo variant above is rejected or correctly classified, with
  tests
- [ ] manifest/build channel mismatch is a named refusal
- [ ] the licence-side parsing is reviewed and the result recorded

## Evidence Required

- the parsing approach and its reason
- the new test batch
- `effigy qa` green

## Stop Conditions

Stop if proper authority parsing pulls in a dependency with its own advisory
surface — that trade belongs in front of the operator, not inside a card.
