# g02.010 Licensing, Entitlement, And Activation

Status: ready
Owner: Tom
Updated: 2026-08-07
Governing refs: contract 019; contracts 004 and 018; research memo 020
Depends on: none within g02

## Outcome

Consuming applications can sell licences on whatever model and whatever
backend they choose. Longhorn owns the licence shape, verification,
entitlement evaluation, lease and grace, and the client surface. It ships no
server and enumerates no features.

## Generation Runway

Tenth g02 milestone, and the second non-remediation one. Shares the adapter
posture of g02.009 deliberately: `ActivationSource` is the same shape as
`UpdateSource`, so a consumer who has integrated one already knows the
other.

The two milestones meet at the update window — the updater asks the licence
whether a release may be taken.

## Execution Plan

### Batch 1. Licence core

- [x] [Card 155](batch-cards/155-licence-model-and-entitlement-evaluation.md)
  builds `longhorn-licence`: verified licence, trust basis, entitlements,
  the two windows, lease and grace, clock-regression refusal

### Batch 2. Acquisition

- [x] [Card 156](batch-cards/156-activation-source-adapters.md) defines
  `ActivationSource` and the signed-file and token-redemption reference
  adapters

### Batch 3. Host and surface

- [ ] [Card 157](batch-cards/157-tauri-licence-host-and-secure-storage.md)
  builds `longhorn-tauri-licence`: secure credential storage, the RFC 8252
  loopback flow, persistence — PKCE, callback validation, machine identity
  and the credential seam are complete; browser launch, platform backend and
  persistence need a packaged proof application
- [ ] [Card 159](batch-cards/159-update-and-licence-packaged-proof.md)
  builds the shared packaged proof application both host cards wait on
- [ ] [Card 193](batch-cards/193-licence-protocol-surface.md) gives the licence
  domain a wire protocol — snapshot, commands, rejection codes, changed event —
  which Card 158 assumed existed
- [ ] [Card 158](batch-cards/158-licence-client-surface.md) builds the licence
  client surface: activation, seat management, expiry. Blocked on Card 193 and
  rescoped: `packages/licence` cannot be built, because g02.013 consolidated
  eighteen packages into three

## Dependency Shape

```text
memo 020 licensing
 └─ 155 licence core ─┬─ 156 activation adapters
                      └─ 157 host and storage ─ 158 client surface
```

156 and 157 are independent of each other. The milestone is independent of
g02.009 except at the update window, which Card 155 models and the updater
reads.

## Goals

- [x] every purchase model expressible without a Longhorn change
- [x] entitlements opaque; Longhorn enumerates no features
- [x] trust basis recorded, and offline grace never granted on a basis that
  cannot survive being offline
- [x] Longhorn answers "entitled?" and never enforces
- [x] an unreachable backend fails open within the lease

## Acceptance Criteria

- [x] subscription, perpetual-with-maintenance, trial, and freemium are all
  expressed in tests using only the two windows, with no model-specific code
- [x] a remote-assertion licence cannot obtain offline grace reserved for
  offline-verifiable licences
- [x] a consumer-implemented adapter inherits evaluation with no extra
  wiring
- [x] a large backwards clock movement is refused
- [ ] licence state refuses a newer schema, per Card 150
- [x] no crate exposes an enforcement call

## Explicit Non-goals

- a licence server, payment handling, tax handling
- signing key custody
- hardware fingerprinting
- EULA presentation
- obfuscation or anti-tamper

## Next Task

Card 193, the licence protocol. Cards 155 and 156 are complete, Card 157's pure
half is done and its host wiring waits on Card 159, and Card 159 is
deprioritized by operator decision — so Card 193 is the only unblocked work in
this milestone.

This section said "Open Card 155" while 155 and 156 were both complete.
