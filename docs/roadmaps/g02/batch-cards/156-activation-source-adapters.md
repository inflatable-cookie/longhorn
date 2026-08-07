# 156 Activation Source Adapters

Status: ready
Owner: Tom
Roadmap: g02.010 batch 2
Governing refs: contract 019; research memo 020
Depends on: Card 155
Auto-start next card: no

## Objective

Define `ActivationSource` and ship reference adapters for signed-file import
and generic token redemption, so a consuming application can put any backend
behind it and inherit evaluation unchanged.

## Scope

- the `ActivationSource` interface: acquire, renew, release
- signed-file import adapter (offline, no network at runtime)
- generic token-redemption adapter
- documented guidance for hosted backends, which stay consumer-implemented

## Steps

1. Define the interface around the three operations a licence actually has a
   lifecycle for: acquire a licence, renew a lease, release an activation
   slot. Self-service release is required by contract 019 and belongs in the
   interface rather than being left to each consumer.
2. Follow `UpdateSource`'s posture from contract 018: describe requests
   where the host will perform them, and keep the crate pure. A consumer who
   has integrated the updater should recognise the shape immediately.
3. Ship the signed-file adapter first. It needs no network at runtime at
   all, which makes it the honest baseline and serves air-gapped and
   procurement-heavy customers.
4. Ship the token-redemption adapter: a short key exchanged for a licence.
   Include the key-format helpers — Crockford base32, grouped, with a check
   character so a typo fails locally rather than after a round trip that
   reads as "you were sold a dud".
5. Each adapter declares its `TrustBasis` honestly. An adapter may not
   present a remote assertion as an offline signature.
6. Document hosted backends as consumer-implemented, with a worked example
   in the test suite rather than a shipped integration. Longhorn takes no
   position on provider.
7. An unreachable source yields the cached licence within its lease, never a
   failure that disables a paying customer.

## Acceptance Criteria

- one interface covers file import, token redemption, and a
  consumer-implemented hosted backend with no special-casing in evaluation
- adapters declare trust basis, and a remote-assertion adapter cannot obtain
  offline-signature grace
- key-format helpers reject a mistyped key locally, and accept wrong case,
  missing dashes, and pasted whitespace
- an unreachable source is not a licence failure within the lease
- release of an activation slot is expressible through the interface

## Evidence Required

- adapter tests including a consumer-implemented hosted source
- key-format acceptance and rejection tests
- unreachable-source behaviour test

## Stop Conditions

- the interface cannot express a backend without leaking that backend's
  concerns into evaluation

## Next Task

Card 157 if not already underway; it does not depend on this card.
