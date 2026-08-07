# 155 Licence Model And Entitlement Evaluation

Status: ready
Owner: Tom
Roadmap: g02.010 batch 1
Governing refs: contract 019; research memo 020
Depends on: none
Auto-start next card: no

## Objective

Build `longhorn-licence`: the verified licence model, trust basis,
entitlement evaluation, the use and update windows, lease and grace, and
clock-regression refusal. Pure — no network, no filesystem, no ambient
clock.

## Scope

- `VerifiedLicence` with entitlements, limits, and the two windows
- `TrustBasis` distinguishing an offline signature from a remote assertion
- entitlement and limit evaluation
- lease, grace, and the fail-open rule
- clock-regression refusal
- Ed25519 verification of a canonical payload

## Steps

1. Model the licence. Two independently optional windows — use and update —
   and nothing named after a product. Prove the four known models in tests
   using only those windows.
2. Model `TrustBasis`. An offline signature is re-verifiable later; a remote
   assertion is a cache with a timestamp. Grace policy consults it, so a
   remote assertion cannot obtain grace reserved for offline verifiability.
3. Entitlements are opaque identifiers. Evaluate presence and limits and
   nothing else. No enumeration, no reserved names, no interpretation.
4. Implement Ed25519 verification over a **canonical** serialisation with
   fixed field order and no floats. Canonicalisation is where these schemes
   fail: verify the bytes as received, never a re-serialisation.
5. Implement lease and grace, including the fail-open rule — an unreachable
   backend within the lease is a valid licence, not an invalid one.
6. Implement clock-regression refusal against a caller-supplied
   highest-seen timestamp. The clock is injected; the crate stays pure.
7. Expose no enforcement call of any kind. The public surface answers
   questions.

## Acceptance Criteria

- subscription, perpetual-with-maintenance, trial, and freemium expressed in
  tests with no model-specific code
- a remote-assertion licence is refused offline grace that an
  offline-signature licence receives
- a tampered payload fails verification; a re-serialisation round trip does
  not change the verified bytes
- an unreachable backend within the lease evaluates as valid
- a large backwards clock movement is refused
- no public function enforces, disables, or degrades anything
- pure crate: no network, no filesystem, no ambient clock

## Evidence Required

- the four purchase models as tests
- the trust-basis grace distinction as a test
- canonicalisation and tamper tests
- fail-open and clock-regression tests

## Stop Conditions

- the two windows cannot express a model a consumer needs, which would mean
  the abstraction is wrong rather than incomplete

## Next Task

Cards 156 and 157, which are independent of each other.
